use ic_asset_certification::{ Asset, AssetConfig, AssetEncoding, AssetRouter };
use ic_cdk::api::set_certified_data;
use ic_http_certification::{
    HeaderField,
    HttpCertification,
    HttpCertificationPath,
    HttpCertificationTree,
    HttpCertificationTreeEntry,
};
use std::{ cell::RefCell, rc::Rc };

use crate::state::read_state;

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

const IMMUTABLE_ASSET_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
pub const NO_CACHE_ASSET_CACHE_CONTROL: &str = "public, no-cache, no-store";

fn get_asset_config() -> Vec<AssetConfig> {
    // 1. Define the asset certification configurations.
    let encodings = vec![
        AssetEncoding::Brotli.default_config(),
        AssetEncoding::Gzip.default_config()
    ];

    let asset_configs = vec![
        AssetConfig::Pattern {
            pattern: "**/*.png".to_string(),
            content_type: Some("image/png".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.jpeg".to_string(),
            content_type: Some("image/jpeg".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.jpg".to_string(),
            content_type: Some("image/jpeg".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.gif".to_string(),
            content_type: Some("image/gif".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.svg".to_string(),
            content_type: Some("image/svg+xml".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.mp3".to_string(),
            content_type: Some("audio/mpeg".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: vec![],
        },
        AssetConfig::Pattern {
            pattern: "**/*.mp4".to_string(),
            content_type: Some("video/mp4".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: vec![],
        },
        AssetConfig::Pattern {
            pattern: "**/*.html".to_string(),
            content_type: Some("text/html".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), NO_CACHE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.css".to_string(),
            content_type: Some("text/css".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), NO_CACHE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.js".to_string(),
            content_type: Some("application/javascript".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), NO_CACHE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.json".to_string(),
            content_type: Some("application/json".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), NO_CACHE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.xml".to_string(),
            content_type: Some("application/xml".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), NO_CACHE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.txt".to_string(),
            content_type: Some("text/plain".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), NO_CACHE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.woff".to_string(),
            content_type: Some("font/woff".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.woff2".to_string(),
            content_type: Some("font/woff2".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.ttf".to_string(),
            content_type: Some("font/ttf".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.eot".to_string(),
            content_type: Some("application/vnd.ms-fontobject".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.otf".to_string(),
            content_type: Some("font/otf".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        },
        AssetConfig::Pattern {
            pattern: "**/*.ico".to_string(),
            content_type: Some("image/x-icon".to_string()), // updated content_type
            headers: get_asset_headers(
                vec![("cache-control".to_string(), IMMUTABLE_ASSET_CACHE_CONTROL.to_string())]
            ),
            encodings: encodings.clone(),
        }
        // AssetConfig::Redirect {
        //     from: "/old-url".to_string(),
        //     to: "/".to_string(),
        //     kind: AssetRedirectKind::Permanent,
        //     headers: get_asset_headers(
        //         vec![
        //             ("content-type".to_string(), "text/plain".to_string()),
        //             ("cache-control".to_string(), NO_CACHE_ASSET_CACHE_CONTROL.to_string())
        //         ]
        //     ),
        // }
    ];

    asset_configs
}

pub fn certify_asset(assets: Vec<Asset<'static, '_>>) {
    let asset_configs = get_asset_config();

    ASSET_ROUTER.with_borrow_mut(|asset_router| {
        // 4. Certify the assets using the `certify_assets` function from the `ic-asset-certification` crate.
        if let Err(err) = asset_router.certify_assets(assets, asset_configs) {
            ic_cdk::trap(&format!("Failed to certify assets: {}", err));
        }

        // 5. Set the canister's certified data.
        set_certified_data(&asset_router.root_hash());
    });
}

pub fn uncertify_asset(assets: Vec<Asset<'static, '_>>) {
    let asset_configs = get_asset_config();

    ASSET_ROUTER.with_borrow_mut(|asset_router| {
        // 4. Certify the assets using the `certify_assets` function from the `ic-asset-certification` crate.
        if let Err(err) = asset_router.delete_assets(assets, asset_configs) {
            ic_cdk::trap(&format!("Failed to certify assets: {}", err));
        }

        // 5. Set the canister's certified data.
        set_certified_data(&asset_router.root_hash());
    });
}

// Certification
pub fn certify_all_assets() {
    let asset_configs = get_asset_config();
    // 2. Collect all assets from the frontend build directory.
    let mut assets = Vec::new();
    read_state(|state| {
        for (internal_metadata, raw_content) in state.data.storage.get_all_files() {
            assets.push(Asset::new(internal_metadata.file_path.clone(), raw_content.clone()));
        }
    });

    // 3. Skip certification for the metrics endpoint.
    HTTP_TREE.with(|tree| {
        let mut tree = tree.borrow_mut();

        let metrics_tree_path = HttpCertificationPath::exact("/metrics");
        let metrics_certification = HttpCertification::skip();
        let metrics_tree_entry = HttpCertificationTreeEntry::new(
            metrics_tree_path,
            metrics_certification
        );
        tree.insert(&metrics_tree_entry);

        let logs_tree_path = HttpCertificationPath::exact("/logs");
        let logs_certification = HttpCertification::skip();
        let logs_tree_entry = HttpCertificationTreeEntry::new(logs_tree_path, logs_certification);
        tree.insert(&logs_tree_entry);

        let trace_tree_path = HttpCertificationPath::exact("/trace");
        let trace_certification = HttpCertification::skip();
        let trace_tree_entry = HttpCertificationTreeEntry::new(
            trace_tree_path,
            trace_certification
        );
        tree.insert(&trace_tree_entry);
    });

    ASSET_ROUTER.with_borrow_mut(|asset_router| {
        // 4. Certify the assets using the `certify_assets` function from the `ic-asset-certification` crate.
        if let Err(err) = asset_router.certify_assets(assets, asset_configs) {
            ic_cdk::trap(&format!("Failed to certify assets: {}", err));
        }

        // 5. Set the canister's certified data.
        set_certified_data(&asset_router.root_hash());
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
