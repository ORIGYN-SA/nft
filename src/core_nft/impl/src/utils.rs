use crate::state::{read_state, RuntimeState};
use candid::{Nat, Principal};
use core_nft_common::types::icrc7;
use core_nft_common::types::management::update_nft_metadata_batch::{
    Item, LogMode, MAX_METADATA_BATCH_ITEMS,
};
use core_nft_common::utils::trace;
use icrc_ledger_types::icrc::generic_value::ICRC3Value as Icrc3Value;
use serde_bytes::ByteBuf;
use std::collections::{BTreeMap, BTreeSet};

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

/// Refuses an `update_nft_metadata_batch` the call must not start: an empty
/// batch, one over `MAX_METADATA_BATCH_ITEMS`, or one naming a token twice.
/// The count is checked first, so the repeat check works on a bounded batch.
pub fn check_metadata_batch(items: &[Item]) -> Result<(), String> {
    if items.is_empty() {
        return Err("empty batch".to_string());
    }
    if items.len() > MAX_METADATA_BATCH_ITEMS {
        return Err(format!(
            "batch of {} items exceeds the limit of {}",
            items.len(),
            MAX_METADATA_BATCH_ITEMS
        ));
    }
    let mut named = BTreeSet::new();
    if !items.iter().all(|item| named.insert(&item.token_id)) {
        return Err("duplicate token id in batch".to_string());
    }
    Ok(())
}

/// The `meta` of a `7update_token` block.
///
/// `Full` logs both metadata maps. `HashOnly` logs the ICRC-3
/// representation-independent hash of each map instead (`ICRC3Value::hash`),
/// so the previous values stay out of the permanent log. The private
/// collections plan consumes these hashes; indexers only store them.
pub fn update_token_meta(previous: Icrc3Value, new: Icrc3Value, log: LogMode) -> Icrc3Value {
    let mut meta = BTreeMap::new();
    match log {
        LogMode::Full => {
            meta.insert("icrc7:previous_metadata".to_string(), previous);
            meta.insert("icrc7:new_metadata".to_string(), new);
        }
        LogMode::HashOnly => {
            meta.insert(
                "icrc7:previous_metadata_hash".to_string(),
                Icrc3Value::Blob(ByteBuf::from(previous.hash().to_vec())),
            );
            meta.insert(
                "icrc7:new_metadata_hash".to_string(),
                Icrc3Value::Blob(ByteBuf::from(new.hash().to_vec())),
            );
        }
    }
    Icrc3Value::Map(meta)
}

#[cfg(test)]
mod tests {
    use super::{check_metadata_batch, update_token_meta};
    use candid::Nat;
    use core_nft_common::types::management::update_nft_metadata_batch::{
        Item, LogMode, MAX_METADATA_BATCH_ITEMS,
    };
    use icrc_ledger_types::icrc::generic_value::ICRC3Value;
    use std::collections::BTreeMap;

    fn map(entries: &[(&str, &str)]) -> ICRC3Value {
        ICRC3Value::Map(
            entries
                .iter()
                .map(|(key, text)| (key.to_string(), ICRC3Value::Text(text.to_string())))
                .collect(),
        )
    }

    fn entries(meta: ICRC3Value) -> BTreeMap<String, ICRC3Value> {
        match meta {
            ICRC3Value::Map(entries) => entries,
            other => panic!("meta is not a map: {:?}", other),
        }
    }

    fn blob_hex(value: &ICRC3Value) -> String {
        match value {
            ICRC3Value::Blob(bytes) => hex::encode(bytes),
            other => panic!("not a blob: {:?}", other),
        }
    }

    #[test]
    fn full_meta_keeps_both_maps() {
        let previous = map(&[("JSON_DATA", r#"{"a":1}"#)]);
        let new = map(&[("JSON_DATA", r#"{"a":2}"#)]);

        let meta = entries(update_token_meta(
            previous.clone(),
            new.clone(),
            LogMode::Full,
        ));

        assert_eq!(meta.len(), 2);
        assert_eq!(meta["icrc7:previous_metadata"], previous);
        assert_eq!(meta["icrc7:new_metadata"], new);
    }

    // Expected hashes were computed outside Rust from the ICRC-3 rule for a
    // map: SHA-256 over the sorted pairs sha256(key) || sha256(value).
    #[test]
    fn hash_only_meta_holds_the_map_hashes() {
        let meta = entries(update_token_meta(
            map(&[("JSON_DATA", r#"{"a":1}"#)]),
            map(&[("JSON_DATA", r#"{"a":2}"#)]),
            LogMode::HashOnly,
        ));

        assert_eq!(meta.len(), 2);
        assert_eq!(
            blob_hex(&meta["icrc7:previous_metadata_hash"]),
            "f8944a61e286758f420f491068825878d24424a5168614ea435fb894a298918a"
        );
        assert_eq!(
            blob_hex(&meta["icrc7:new_metadata_hash"]),
            "aef744412f4848b03b40fccf42d6855f6e61f336f8d76b8b43da29a85af51ee4"
        );
    }

    #[test]
    fn hash_only_hash_covers_every_key() {
        let meta = entries(update_token_meta(
            map(&[("JSON_DATA", r#"{"a":1}"#), ("name", "x")]),
            map(&[]),
            LogMode::HashOnly,
        ));

        assert_eq!(
            blob_hex(&meta["icrc7:previous_metadata_hash"]),
            "3d0e881aad4984eb7f5fa5ddaaae95a0960e9e084321d5d43c5c39e55d5fc6b8"
        );
    }

    #[test]
    fn hash_only_hash_of_an_empty_map_is_sha256_of_nothing() {
        let meta = entries(update_token_meta(
            map(&[]),
            map(&[("JSON_DATA", r#"{"a":1}"#)]),
            LogMode::HashOnly,
        ));

        assert_eq!(
            blob_hex(&meta["icrc7:previous_metadata_hash"]),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    fn items(token_ids: impl IntoIterator<Item = u64>) -> Vec<Item> {
        token_ids
            .into_iter()
            .map(|token_id| Item {
                token_id: Nat::from(token_id),
                metadata: vec![],
            })
            .collect()
    }

    #[test]
    fn batch_check_refuses_an_empty_batch() {
        assert_eq!(check_metadata_batch(&[]), Err("empty batch".to_string()));
    }

    #[test]
    fn batch_check_takes_the_largest_batch() {
        assert_eq!(MAX_METADATA_BATCH_ITEMS, 50);
        assert_eq!(check_metadata_batch(&items(1..=50)), Ok(()));
    }

    #[test]
    fn batch_check_refuses_one_item_past_the_limit() {
        assert_eq!(
            check_metadata_batch(&items(1..=51)),
            Err("batch of 51 items exceeds the limit of 50".to_string())
        );
    }

    #[test]
    fn batch_check_refuses_a_repeated_token_id() {
        assert_eq!(
            check_metadata_batch(&items([3, 1, 2, 1])),
            Err("duplicate token id in batch".to_string())
        );
    }
}
