use std::net::{IpAddr, Ipv4Addr};

use crate::secret::hash_secret;

const UNKNOWN_IP: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
const MAX_KEY_LENGTH: usize = 254;

pub(super) fn ip_key(ip: Option<IpAddr>) -> String {
    ip.unwrap_or(UNKNOWN_IP).to_string()
}

pub(super) fn failed_login_key(email: &str, ip: Option<IpAddr>) -> String {
    format!("{} {}", account_key(email), ip_key(ip))
}

pub(super) fn account_key(email: &str) -> String {
    bounded(email.trim().to_lowercase())
}

pub(super) fn client_key(client_id: &str) -> String {
    bounded(client_id.to_string())
}

fn bounded(key: String) -> String {
    if key.len() <= MAX_KEY_LENGTH {
        key
    } else {
        hash_secret(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_normalized_and_long_ones_hashed() {
        assert_eq!(account_key("  Ada@Example.TEST "), "ada@example.test");
        assert_eq!(client_key("short"), "short");
        let long = "x".repeat(MAX_KEY_LENGTH + 1);
        assert_eq!(client_key(&long), hash_secret(&long));
        assert!(account_key(&long).len() <= MAX_KEY_LENGTH);
        assert_eq!(ip_key(None), "0.0.0.0");
    }
}
