use bity_ic_canister_logger::LogEntry;
use bity_ic_storage_canister_api::types::storage::UploadState;
use candid::Principal;
use core_nft_common::types::http::{
    add_redirection, get_asset_headers, ASSET_ROUTER, HTTP_TREE, NO_CACHE_ASSET_CACHE_CONTROL,
};
use ic_cdk::api::data_certificate;
use ic_cdk::update;
use ic_cdk_macros::query;
use ic_http_certification::{
    utils::add_v2_certificate_header, DefaultCelBuilder, HttpCertification, HttpCertificationPath,
    HttpCertificationTreeEntry, HttpRequest, HttpResponse, HttpUpdateRequest, HttpUpdateResponse,
    StatusCode, CERTIFICATE_EXPRESSION_HEADER_NAME,
};

use crate::state::{mutate_state, read_state, RuntimeState};
use crate::utils::{normalize_media_path, render_media_url_from_state};

#[query(hidden = true)]
fn http_request(req: HttpRequest) -> HttpResponse<'static> {
    let path = req.get_path().expect("Failed to parse request path");

    match path.as_str() {
        "/logs" => serve_logs(bity_ic_canister_logger::export_logs()),
        "/traces" => serve_logs(bity_ic_canister_logger::export_traces()),
        "/metrics" => serve_metrics(),
        _ => serve_media(&req, &path),
    }
}

/// Answers a request for a stored file.
///
/// This canister holds no file bytes: they live in its storage sub-canisters,
/// and a request is answered with a redirect to the one holding the file.
/// Normally that redirect is already certified in the asset router, registered
/// by `finalize_upload`. When it is not, the canister still knows where the
/// file is from its own bookkeeping, so the path is resolved rather than failed.
fn serve_media(req: &HttpRequest, path: &str) -> HttpResponse<'static> {
    if let Some(response) = serve_asset(req) {
        return response;
    }

    if is_raw_request(req) {
        // Nothing is certification-verified on the raw domain, so a redirect
        // built here is served as-is.
        match read_state(|state| resolve_media_url(state, path)) {
            Some(url) => redirect_to(url),
            None => not_found(),
        }
    } else {
        // On the certified domain an uncertified response is rejected by the
        // boundary node, and that includes a 404. Hand the request to the
        // update call, whose response goes through consensus and is trusted.
        HttpResponse::builder().with_upgrade(true).build()
    }
}

/// Resolves and certifies a media path that was missing from the asset router.
///
/// Registering the redirect here is what makes the miss self-healing: the next
/// request for the same path is served straight from the router, and persisting
/// it in `media_redirections` means it also survives the next upgrade, since
/// `post_upgrade` rebuilds the router from that map.
#[update(hidden = true)]
async fn http_request_update(req: HttpUpdateRequest<'static>) -> HttpUpdateResponse<'static> {
    let path = req.get_path().expect("Failed to parse request path");

    let Some(url) = read_state(|state| resolve_media_url(state, &path)) else {
        return HttpUpdateResponse::from(not_found());
    };

    let key = normalize_media_path(&path);
    add_redirection(key.clone(), url.clone());
    mutate_state(|state| {
        state.data.media_redirections.insert(key, url.clone());
    });

    HttpUpdateResponse::from(redirect_to(url))
}

/// Where a stored file is served from, or `None` if this collection does not
/// know the path.
fn resolve_media_url(state: &RuntimeState, path: &str) -> Option<String> {
    let slashed = normalize_media_path(path);
    let bare = path.trim_start_matches('/').to_string();

    // An already-rendered target wins: it is exactly what finalize_upload
    // handed out, so a stale base_url template cannot make it drift.
    if let Some(url) = state
        .data
        .media_redirections
        .get(&slashed)
        .or_else(|| state.data.media_redirections.get(&bare))
    {
        return Some(url.clone());
    }

    let storage_canister_id = resolve_storage_canister(state, &slashed, &bare)?;
    Some(render_media_url_from_state(
        state,
        storage_canister_id,
        &slashed,
    ))
}

/// Finds the storage sub-canister holding `path`.
///
/// Paths are looked up both with and without their leading slash: the router
/// and `media_redirections` are keyed leading-slashed, but the content
/// bookkeeping is keyed by the upload's original `file_path`, which may not be.
fn resolve_storage_canister(state: &RuntimeState, slashed: &str, bare: &str) -> Option<Principal> {
    for key in [slashed, bare] {
        if let Some(entry) = state
            .data
            .public_content_system
            .get_public_file_by_path(key)
        {
            if entry.state == UploadState::Finalized {
                return Some(entry.storage_canister_id);
            }
        }
    }

    // Uploads from before the content system existed.
    for key in [slashed, bare] {
        if let Some(data) = state.internal_filestorage.get(key) {
            if data.state == UploadState::Finalized {
                return Some(data.canister);
            }
        }
    }

    let matches = |candidate: &str| candidate == slashed || candidate == bare;

    for record in state.data.private_content_system.nft_private.values() {
        for entry in record.entries.values() {
            if matches(&entry.storage_path) {
                return Some(entry.storage_canister_id);
            }
        }
    }
    for entry in state.data.private_content_system.temp_file_cache.values() {
        if matches(&entry.storage_path) {
            return Some(entry.storage_canister_id);
        }
    }

    None
}

fn is_raw_request(req: &HttpRequest) -> bool {
    req.headers()
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("host") && v.contains(".raw."))
}

fn redirect_to(url: String) -> HttpResponse<'static> {
    HttpResponse::temporary_redirect(
        url,
        get_asset_headers(vec![
            (
                "cache-control".to_string(),
                NO_CACHE_ASSET_CACHE_CONTROL.to_string(),
            ),
            ("content-type".to_string(), "text/plain".to_string()),
        ]),
    )
    .build()
}

fn not_found() -> HttpResponse<'static> {
    HttpResponse::builder()
        .with_status_code(StatusCode::NOT_FOUND)
        .build()
}

fn serve_logs(logs: Vec<LogEntry>) -> HttpResponse<'static> {
    ASSET_ROUTER.with_borrow(|_| {
        let body = serde_json::to_vec(&logs).expect("Failed to serialize metrics");
        let headers = get_asset_headers(vec![
            (
                CERTIFICATE_EXPRESSION_HEADER_NAME.to_string(),
                DefaultCelBuilder::skip_certification().to_string(),
            ),
            ("content-type".to_string(), "application/json".to_string()),
            (
                "cache-control".to_string(),
                NO_CACHE_ASSET_CACHE_CONTROL.to_string(),
            ),
        ]);
        let mut response = HttpResponse::builder()
            .with_status_code(StatusCode::OK)
            .with_body(body)
            .with_headers(headers)
            .build();

        HTTP_TREE.with(|tree| {
            let tree = tree.borrow();

            let metrics_tree_path = HttpCertificationPath::exact("/metrics");
            let metrics_certification = HttpCertification::skip();
            let metrics_tree_entry =
                HttpCertificationTreeEntry::new(&metrics_tree_path, metrics_certification);
            add_v2_certificate_header(
                &data_certificate().expect("No data certificate available"),
                &mut response,
                &tree.witness(&metrics_tree_entry, "/metrics").unwrap(),
                &metrics_tree_path.to_expr_path(),
            );

            response
        })
    })
}

fn serve_metrics() -> HttpResponse<'static> {
    ASSET_ROUTER.with_borrow(|_| {
        let metrics = read_state(|state| state.metrics());
        let body = serde_json::to_vec(&metrics).expect("Failed to serialize metrics");
        let headers = get_asset_headers(vec![
            (
                CERTIFICATE_EXPRESSION_HEADER_NAME.to_string(),
                DefaultCelBuilder::skip_certification().to_string(),
            ),
            ("content-type".to_string(), "application/json".to_string()),
            (
                "cache-control".to_string(),
                NO_CACHE_ASSET_CACHE_CONTROL.to_string(),
            ),
        ]);
        let mut response = HttpResponse::builder()
            .with_status_code(StatusCode::OK)
            .with_body(body)
            .with_headers(headers)
            .build();

        HTTP_TREE.with(|tree| {
            let tree = tree.borrow();

            let metrics_tree_path = HttpCertificationPath::exact("/metrics");
            let metrics_certification = HttpCertification::skip();
            let metrics_tree_entry =
                HttpCertificationTreeEntry::new(&metrics_tree_path, metrics_certification);
            add_v2_certificate_header(
                &data_certificate().expect("No data certificate available"),
                &mut response,
                &tree.witness(&metrics_tree_entry, "/metrics").unwrap(),
                &metrics_tree_path.to_expr_path(),
            );

            response
        })
    })
}

/// Serves a redirect already certified in the asset router.
///
/// Returns `None` on a miss rather than trapping. A trap surfaces as a 503,
/// which is indistinguishable from the canister being down and leaves the
/// caller no way to tell an unknown path from an outage.
fn serve_asset(req: &HttpRequest) -> Option<HttpResponse<'static>> {
    ASSET_ROUTER.with_borrow(|asset_router| {
        let data_cert = data_certificate().expect("No data certificate available");
        asset_router.serve_asset(&data_cert, req).ok()
    })
}
