use std::str::FromStr;
use types::TimestampMillis;

pub enum Route {
    Logs(Option<TimestampMillis>),
    Traces(Option<TimestampMillis>),
    Metrics,
    Other(String, String),
}

pub fn extract_route(path: &str) -> Route {
    let trimmed = path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .to_lowercase();

    if trimmed.is_empty() {
        return Route::Other("".to_string(), "".to_string());
    }

    let (path, qs) = trimmed.split_once('?').unwrap_or((&trimmed, ""));

    let parts: Vec<_> = path.split('/').collect();

    match parts[0] {
        "logs" => {
            let since = parts.get(1).and_then(|p| u64::from_str(p).ok());
            return Route::Logs(since);
        }
        "trace" => {
            let since = parts.get(1).and_then(|p| u64::from_str(p).ok());
            return Route::Traces(since);
        }
        "metrics" => {
            return Route::Metrics;
        }
        _ => (),
    }

    Route::Other(path.to_string(), qs.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs() {
        assert!(matches!(
            extract_route("/logs/1633649663014109000"),
            Route::Logs(_)
        ));
    }

    #[test]
    fn other() {
        assert!(matches!(extract_route("blah"), Route::Other(_, _)));
    }

    #[test]
    fn querystring() {
        let route = extract_route("blah?abc=1");
        if let Route::Other(p, qs) = route {
            assert_eq!(&p, "blah");
            assert_eq!(&qs, "abc=1");
        } else {
            panic!();
        }
    }
}
