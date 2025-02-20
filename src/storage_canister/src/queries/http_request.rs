use http_request::{ build_json_response, encode_logs, extract_route, Route };
use ic_cdk_macros::query;
use types::{
    HeaderField,
    HttpRequest,
    HttpResponse,
    StreamingStrategy,
    TimestampMillis,
    Token,
    CallbackFunc,
};
use serde_bytes::ByteBuf;
use candid::Nat;

use crate::{ state::{ read_state, RuntimeState }, utils::trace };

const CHUNK_SIZE: usize = 1_500_000;

fn get_file_chunk(state: &RuntimeState, key: String, index: Nat) -> Result<HttpResponse, String> {
    match state.data.get_raw_data(key.clone()) {
        Ok((internal_inf, raw)) => {
            let start = usize::try_from((index.clone() * Nat::from(CHUNK_SIZE)).0).unwrap();
            let end = usize
                ::try_from(((index.clone() + Nat::from(1 as u64)) * Nat::from(CHUNK_SIZE)).0)
                .unwrap_or(raw.len());
            let content_range = format!("bytes {}-{}/{}", start, end.min(raw.len()) - 1, raw.len());
            let body = if start < raw.len() {
                ByteBuf::from(raw[start..end.min(raw.len())].to_vec())
            } else {
                ByteBuf::from(vec![])
            };
            let status_code = if start < raw.len() { 206 } else { 200 };
            trace(&format!("get_file_chunk - content_range: {:?}", content_range));
            let response = HttpResponse {
                status_code,
                headers: vec![
                    HeaderField("Content-Type".to_string(), internal_inf.file_type.clone().into()),
                    HeaderField("Content-Length".to_string(), body.len().to_string()), // Updated to chunk length
                    HeaderField("Content-Range".to_string(), content_range)
                ],
                body,
                streaming_strategy: if start < raw.len() {
                    Some(StreamingStrategy::Callback {
                        callback: CallbackFunc::new(
                            ic_cdk::id(),
                            "http_request_raw_callback".to_string()
                        ),
                        token: Token {
                            key: key.clone(),
                            content_encoding: internal_inf.file_type.clone(),
                            index: index.clone() + Nat::from(1 as u64),
                            sha256: None,
                        },
                    })
                } else {
                    None
                },
            };
            Ok(response)
        }
        Err(err) => Err(format!("get_file_chunk - error: {:?}", err)),
    }
}

#[query(hidden = true)]
fn http_request(request: HttpRequest) -> HttpResponse {
    trace(&format!("http_request: {:?}", request));

    fn get_logs_impl(since: Option<TimestampMillis>) -> HttpResponse {
        encode_logs(canister_logger::export_logs(), since.unwrap_or(0))
    }

    fn get_traces_impl(since: Option<TimestampMillis>) -> HttpResponse {
        encode_logs(canister_logger::export_traces(), since.unwrap_or(0))
    }

    fn get_metrics_impl(state: &RuntimeState) -> HttpResponse {
        build_json_response(&state.metrics())
    }

    fn get_file(state: &RuntimeState, path: String) -> HttpResponse {
        trace(&format!("get_file: {}", path));
        let parts: Vec<&str> = path.split('/').collect();
        trace(&format!("get_file - parts: {:?}", parts));
        if parts.len() == 1 {
            match get_file_chunk(state, parts[0].to_string(), Nat::from(0 as u64)) {
                Ok(response) => response,
                Err(err) => {
                    trace(&err);
                    HttpResponse::not_found()
                }
            }
        } else {
            trace("get_file - invalid path length");
            HttpResponse::not_found()
        }
    }

    match extract_route(&request.url) {
        Route::Logs(since) => get_logs_impl(since),
        Route::Traces(since) => get_traces_impl(since),
        Route::Metrics => read_state(get_metrics_impl),
        Route::Other(path, _) => {
            return read_state(|state| get_file(state, path));
        }
    }
}

#[query(hidden = true)]
fn http_request_raw_callback(request: Token) -> HttpResponse {
    trace(&format!("http_request_raw_callback: {:?}", request));
    read_state(|state| {
        match get_file_chunk(state, request.key.clone(), request.index.clone()) {
            Ok(response) => response,
            Err(err) => {
                trace(&err);
                HttpResponse::not_found()
            }
        }
    })
}
