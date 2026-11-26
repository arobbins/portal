use url::{Host, Url};

pub const MAX_REDIRECT_URI_LENGTH: usize = 2048;
pub const MAX_REDIRECT_URIS: usize = 10;

pub fn is_allowed_redirect(uri: &str) -> bool {
    if uri.len() > MAX_REDIRECT_URI_LENGTH {
        return false;
    }
    let Ok(url) = Url::parse(uri) else {
        return false;
    };
    if url.fragment().is_some() || !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    match (url.scheme(), url.host()) {
        ("https", Some(_)) => true,
        ("http", Some(host)) => is_loopback(&host),
        _ => false,
    }
}

fn is_loopback(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(name) => name.eq_ignore_ascii_case("localhost"),
        Host::Ipv4(ip) => ip.is_loopback(),
        Host::Ipv6(ip) => ip.is_loopback(),
    }
}

pub fn return_host(uri: &str) -> String {
    Url::parse(uri)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .map(|host| {
            host.trim_start_matches('[')
                .trim_end_matches(']')
                .to_string()
        })
        .unwrap_or_default()
}

pub fn with_params(uri: &str, params: &[(&str, &str)]) -> String {
    let Ok(mut url) = Url::parse(uri) else {
        return uri.to_string();
    };
    url.query_pairs_mut().extend_pairs(params);
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_or_loopback_http_may_be_returned_to() {
        for allowed in [
            "https://claude.ai/api/mcp/auth_callback",
            "http://127.0.0.1:33418/callback",
            "http://localhost:8080/cb",
            "http://[::1]:9000/cb",
        ] {
            assert!(is_allowed_redirect(allowed), "{allowed}");
        }
        for refused in [
            "http://claude.ai/callback",
            "javascript:alert(1)",
            "cursor://callback",
            "https://claude.ai/cb#fragment",
            "https://user:pass@claude.ai/cb",
            "not a url",
        ] {
            assert!(!is_allowed_redirect(refused), "{refused}");
        }
        let long = format!(
            "https://example.com/{}",
            "a".repeat(MAX_REDIRECT_URI_LENGTH)
        );
        assert!(!is_allowed_redirect(&long));
    }

    #[test]
    fn the_return_host_has_no_port() {
        assert_eq!(return_host("https://claude.ai/api/cb"), "claude.ai");
        assert_eq!(return_host("http://127.0.0.1:33418/callback"), "127.0.0.1");
        assert_eq!(return_host("http://[::1]:9000/cb"), "::1");
    }

    #[test]
    fn params_are_added_to_the_existing_query() {
        assert_eq!(
            with_params(
                "https://app.test/cb?x=1",
                &[("code", "a b"), ("state", "s")]
            ),
            "https://app.test/cb?x=1&code=a+b&state=s"
        );
    }
}
