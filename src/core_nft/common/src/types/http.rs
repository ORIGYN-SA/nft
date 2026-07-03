use ic_asset_certification::{AssetConfig, AssetRedirectKind, AssetRouter};
use ic_cdk::api::certified_data_set;
use ic_http_certification::{
    HeaderField, HttpCertification, HttpCertificationPath, HttpCertificationTree,
    HttpCertificationTreeEntry,
};
use std::{cell::RefCell, rc::Rc};

use crate::utils::trace;

thread_local! {
    pub static HTTP_TREE: Rc<RefCell<HttpCertificationTree>> = Default::default();

    // initializing the asset router with an HTTP certification tree is optional.
    // if direct access to the HTTP certification tree is not needed for certifying
    // requests and responses outside of the asset router, then this step can be skipped
    // and the asset router can be initialized like so:
    // ```
    // static ASSET_ROUTER: RefCell<AssetRouter<'static>> = Default::default();
    // ```
    pub static ASSET_ROUTER: RefCell<AssetRouter<'static>> = RefCell::new(
        AssetRouter::with_tree(HTTP_TREE.with(|tree| tree.clone()))
    );
}

// const IMMUTABLE_ASSET_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
pub const NO_CACHE_ASSET_CACHE_CONTROL: &str = "public, no-cache, no-store";

pub fn add_redirection(from_url: String, to_url: String) -> Option<()> {
    trace(&format!(
        "add_redirection: from_url: {}, to_url: {}",
        from_url, to_url
    ));
    let asset_configs = vec![AssetConfig::Redirect {
        from: from_url,
        to: to_url,
        kind: AssetRedirectKind::Temporary,
        headers: get_asset_headers(vec![
            ("content-type".to_string(), "text/plain".to_string()),
            (
                "cache-control".to_string(),
                NO_CACHE_ASSET_CACHE_CONTROL.to_string(),
            ),
        ]),
    }];

    ASSET_ROUTER.with_borrow_mut(|asset_router| {
        if let Err(err) = asset_router.certify_assets(vec![], asset_configs) {
            ic_cdk::trap(&format!("Failed to certify assets: {}", err));
        }

        trace(&format!("Certified assets: {:?}", asset_router.root_hash()));

        certified_data_set(&asset_router.root_hash());
    });

    Some(())
}

pub fn remove_redirection(from_url: String, to_url: String) -> Option<()> {
    let asset_configs = vec![AssetConfig::Redirect {
        from: from_url,
        to: to_url,
        kind: AssetRedirectKind::Temporary,
        headers: vec![],
    }];

    ASSET_ROUTER.with_borrow_mut(|asset_router| {
        if let Err(err) = asset_router.delete_assets(vec![], asset_configs) {
            ic_cdk::trap(&format!("Failed to certify assets: {}", err));
        }

        certified_data_set(&asset_router.root_hash());
    });

    Some(())
}

// Certification
pub fn certify_all_assets() {
    // let asset_configs = get_asset_config();

    HTTP_TREE.with(|tree| {
        let mut tree = tree.borrow_mut();

        let metrics_tree_path = HttpCertificationPath::exact("/metrics");
        let metrics_certification = HttpCertification::skip();
        let metrics_tree_entry =
            HttpCertificationTreeEntry::new(metrics_tree_path, metrics_certification);
        tree.insert(&metrics_tree_entry);

        let logs_tree_path = HttpCertificationPath::exact("/logs");
        let logs_certification = HttpCertification::skip();
        let logs_tree_entry = HttpCertificationTreeEntry::new(logs_tree_path, logs_certification);
        tree.insert(&logs_tree_entry);

        let trace_tree_path = HttpCertificationPath::exact("/trace");
        let trace_certification = HttpCertification::skip();
        let trace_tree_entry =
            HttpCertificationTreeEntry::new(trace_tree_path, trace_certification);
        tree.insert(&trace_tree_entry);
    });

    ASSET_ROUTER.with_borrow_mut(|asset_router| {
        certified_data_set(&asset_router.root_hash());
    });
}

pub fn get_asset_headers(additional_headers: Vec<HeaderField>) -> Vec<HeaderField> {
    // set up the default headers and include additional headers provided by the caller
    let mut headers = vec![
        (
            "strict-transport-security".to_string(),
            "max-age=31536000; includeSubDomains".to_string(),
        ),
        ("x-frame-options".to_string(), "DENY".to_string()),
        ("x-content-type-options".to_string(), "nosniff".to_string()),
        (
            "content-security-policy".to_string(),
            "default-src 'self'; img-src 'self' data:; form-action 'self'; object-src 'none'; frame-ancestors 'none'; upgrade-insecure-requests; block-all-mixed-content".to_string(),
        ),
        ("referrer-policy".to_string(), "no-referrer".to_string()),
        (
            "permissions-policy".to_string(),
            "accelerometer=(),ambient-light-sensor=(),autoplay=(),battery=(),camera=(),display-capture=(),document-domain=(),encrypted-media=(),fullscreen=(),gamepad=(),geolocation=(),gyroscope=(),layout-animations=(self),legacy-image-formats=(self),magnetometer=(),microphone=(),midi=(),oversized-images=(self),payment=(),picture-in-picture=(),publickey-credentials-get=(),speaker-selection=(),sync-xhr=(self),unoptimized-images=(self),unsized-media=(self),usb=(),screen-wake-lock=(),web-share=(),xr-spatial-tracking=()".to_string(),
        ),
        ("cross-origin-embedder-policy".to_string(), "require-corp".to_string()),
        ("cross-origin-opener-policy".to_string(), "same-origin".to_string())
    ];
    headers.extend(additional_headers);

    headers
}
