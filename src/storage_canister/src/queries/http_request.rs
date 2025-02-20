use ic_cdk_macros::query;
use ic_http_certification::{
    utils::add_v2_certificate_header,
    DefaultCelBuilder,
    HeaderField,
    HttpCertification,
    HttpCertificationPath,
    HttpCertificationTree,
    HttpCertificationTreeEntry,
    HttpRequest,
    HttpResponse,
    StatusCode,
    CERTIFICATE_EXPRESSION_HEADER_NAME,
};
use crate::types::http::{
    HTTP_TREE,
    ASSET_ROUTER,
    NO_CACHE_ASSET_CACHE_CONTROL,
    get_asset_headers,
};
use ic_cdk::api::data_certificate;

use crate::state::read_state;

#[query(hidden = true)]
fn http_request(req: HttpRequest) -> HttpResponse {
    let path = req.get_path().expect("Failed to parse request path");

    if path == "/metrics" {
        return serve_metrics();
    }

    serve_asset(&req)
}

fn serve_metrics() -> HttpResponse<'static> {
    ASSET_ROUTER.with_borrow(|asset_router| {
        let metrics = read_state(|state| state.metrics());
        let body = serde_json::to_vec(&metrics).expect("Failed to serialize metrics");
        let headers = get_asset_headers(
            vec![
                (
                    CERTIFICATE_EXPRESSION_HEADER_NAME.to_string(),
                    DefaultCelBuilder::skip_certification().to_string(),
                ),
                ("content-type".to_string(), "application/json".to_string()),
                ("cache-control".to_string(), NO_CACHE_ASSET_CACHE_CONTROL.to_string())
            ]
        );
        let mut response = HttpResponse::builder()
            .with_status_code(StatusCode::OK)
            .with_body(body)
            .with_headers(headers)
            .build();

        HTTP_TREE.with(|tree| {
            let tree = tree.borrow();

            let metrics_tree_path = HttpCertificationPath::exact("/metrics");
            let metrics_certification = HttpCertification::skip();
            let metrics_tree_entry = HttpCertificationTreeEntry::new(
                &metrics_tree_path,
                metrics_certification
            );
            add_v2_certificate_header(
                &data_certificate().expect("No data certificate available"),
                &mut response,
                &tree.witness(&metrics_tree_entry, "/metrics").unwrap(),
                &metrics_tree_path.to_expr_path()
            );

            response
        })
    })
}

fn serve_asset(req: &HttpRequest) -> HttpResponse<'static> {
    ASSET_ROUTER.with_borrow(|asset_router| {
        if
            let Ok(response) = asset_router.serve_asset(
                &data_certificate().expect("No data certificate available"),
                req
            )
        {
            response
        } else {
            ic_cdk::trap("Failed to serve asset");
        }
    })
}
