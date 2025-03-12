use crate::{
    state::mutate_state,
    types::http::{self, get_asset_headers, ASSET_ROUTER, HTTP_TREE, NO_CACHE_ASSET_CACHE_CONTROL},
    utils::trace,
};
use canister_logger::LogEntry;
use ic_cdk::api::data_certificate;
use ic_cdk::{trap, update};
use ic_cdk_macros::query;
use ic_http_certification::{
    utils::add_v2_certificate_header, DefaultCelBuilder, HttpCertification, HttpCertificationPath,
    HttpCertificationTreeEntry, HttpRequest, HttpResponse, HttpUpdateRequest, HttpUpdateResponse,
    StatusCode, CERTIFICATE_EXPRESSION_HEADER_NAME,
};

use crate::state::read_state;

#[query(hidden = true)]
async fn http_request(req: HttpRequest<'static>) -> HttpResponse<'static> {
    let path = req.get_path().expect("Failed to parse request path");

    match path.as_str() {
        "/logs" => serve_logs(canister_logger::export_logs()),
        "/traces" => serve_logs(canister_logger::export_traces()),
        "/metrics" => serve_metrics(),
        _ => {
            let asset_resp = serve_asset(&req);

            match asset_resp {
                Some(response) => response,
                None => {
                    if req.headers().to_vec().iter().any(|(k, v)| {
                        k == "referer" && v.contains(ic_cdk::api::id().to_string().as_str())
                    }) {
                        return HttpResponse::builder()
                            .with_status_code(StatusCode::NOT_FOUND)
                            .build();
                    } else {
                        return HttpResponse::builder().with_upgrade(true).build();
                    }
                }
            }
        }
    }
}

#[update(hidden = true)]
async fn http_request_update(req: HttpUpdateRequest<'static>) -> HttpUpdateResponse<'static> {
    let path = req.get_path().expect("Failed to parse request path");

    match path.as_str() {
        _ => {
            trace("Cache miss");
            let cache_miss_ret = mutate_state(|state| state.data.storage.cache_miss(path.clone()));
            match cache_miss_ret {
                Ok(_) => {
                    let redirection_url = format!(
                        "https://{}.raw.icp0.io{}",
                        ic_cdk::api::id().to_string(),
                        path.clone()
                    );

                    let response =
                        HttpResponse::temporary_redirect(redirection_url, req.headers().to_vec())
                            .build();
                    HttpUpdateResponse::from(response)
                }
                Err(e) => {
                    trap(&format!("Failed to cache miss: {:?}", e));
                }
            }
        }
    }
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

fn serve_asset(req: &HttpRequest) -> Option<HttpResponse<'static>> {
    ASSET_ROUTER.with_borrow(|asset_router| {
        let data_cert = data_certificate().expect("No data certificate available");

        if let Ok(response) = asset_router.serve_asset(&data_cert, &req) {
            Some(response)
        } else {
            None
        }
    })
}
