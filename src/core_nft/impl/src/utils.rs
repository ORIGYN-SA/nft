use crate::state::{read_state, RuntimeState};
use candid::{Nat, Principal};
use core_nft_common::types::icrc7;
use core_nft_common::utils::trace;

/// Renders the URL a stored file is served from.
///
/// Every place that hands out or registers a media URL goes through here:
/// `finalize_upload`, `finalize_private_content_upload`, the post-upgrade
/// re-render, and the `http_request` fallback. They have to agree, because the
/// fallback resolves a path by rebuilding the URL that finalize would have
/// produced, and a mismatch would send callers somewhere the file is not.
///
/// `base_url` is a template carrying a `{canister_id}` placeholder, e.g.
/// `https://{canister_id}.raw.icp0.io`. `file_path` is expected to carry its
/// leading slash.
pub fn render_media_url(
    base_url: &Option<String>,
    is_test_mode: bool,
    storage_canister_id: &str,
    file_path: &str,
) -> String {
    match base_url {
        Some(template) => {
            let base = template.replace("{canister_id}", storage_canister_id);
            format!("{}{}", base.trim_end_matches('/'), file_path)
        }
        None => {
            if is_test_mode {
                format!("http://{}.localhost:4943{}", storage_canister_id, file_path)
            } else {
                format!("https://{}.raw.icp0.io{}", storage_canister_id, file_path)
            }
        }
    }
}

/// [render_media_url] resolved against the canister's own configuration.
pub fn render_media_url_from_state(
    state: &RuntimeState,
    storage_canister_id: Principal,
    file_path: &str,
) -> String {
    render_media_url(
        &state.data.base_url,
        state.env.is_test_mode(),
        &storage_canister_id.to_string(),
        file_path,
    )
}

/// A media path in the form the asset router and `media_redirections` are keyed
/// by: always leading-slashed, because `HttpRequest::get_path` always is.
pub fn normalize_media_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    }
}

pub fn check_memo(memo: &Option<serde_bytes::ByteBuf>) -> Result<(), String> {
    if let Some(ref memo) = memo {
        let max_memo_size: usize = usize::try_from(
            read_state(|state| {
                state
                    .data
                    .max_memo_size
                    .clone()
                    .unwrap_or(Nat::from(icrc7::DEFAULT_MAX_MEMO_SIZE))
            })
            .0,
        )
        .unwrap();

        trace(&format!("Memo Size: {}", memo.len()));
        trace(&format!("Max Memo Size: {}", max_memo_size));

        if memo.len() > max_memo_size {
            trace("Exceeds Max Memo Size");
            return Err("Exceeds Max Memo Size".to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {}
