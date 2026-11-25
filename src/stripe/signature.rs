use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const SIGNATURE_TOLERANCE_SECONDS: i64 = 5 * 60;

pub fn verify_signature(secret: &str, header: Option<&str>, payload: &[u8], now: i64) -> bool {
    let Some(header) = header else {
        return false;
    };
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", value)) => timestamp = value.parse::<i64>().ok(),
            Some(("v1", value)) => signatures.extend(hex::decode(value).ok()),
            _ => {}
        }
    }
    let Some(timestamp) = timestamp else {
        return false;
    };
    if (now - timestamp).abs() > SIGNATURE_TOLERANCE_SECONDS {
        return false;
    }
    signatures
        .iter()
        .any(|signature| signed_by(secret, timestamp, payload, signature))
}

fn signed_by(secret: &str, timestamp: i64, payload: &[u8], signature: &[u8]) -> bool {
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    mac.verify_slice(signature).is_ok()
}

#[cfg(test)]
pub fn sign(secret: &str, timestamp: i64, payload: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(format!("{timestamp}.").as_bytes());
    mac.update(payload);
    format!(
        "t={timestamp},v1={}",
        hex::encode(mac.finalize().into_bytes())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "whsec_test_secret";
    const NOW: i64 = 1_800_000_000;

    #[test]
    fn a_signature_from_the_secret_is_accepted() {
        let header = sign(SECRET, NOW, b"{}");

        assert!(verify_signature(SECRET, Some(&header), b"{}", NOW + 10));
    }

    #[test]
    fn missing_wrong_stale_or_altered_signatures_are_refused() {
        let header = sign(SECRET, NOW, b"{}");
        let wrong = sign("whsec_other", NOW, b"{}");
        let stale = sign(SECRET, NOW - SIGNATURE_TOLERANCE_SECONDS - 1, b"{}");

        assert!(!verify_signature(SECRET, None, b"{}", NOW));
        assert!(!verify_signature(SECRET, Some(&wrong), b"{}", NOW));
        assert!(!verify_signature(SECRET, Some(&stale), b"{}", NOW));
        assert!(!verify_signature(SECRET, Some(&header), b"{ }", NOW));
        assert!(!verify_signature(SECRET, Some("t=x,v1=zz"), b"{}", NOW));
    }

    #[test]
    fn any_of_several_v1_signatures_may_match() {
        let good = sign(SECRET, NOW, b"{}");
        let v1 = good.split("v1=").nth(1).unwrap();
        let header = format!("t={NOW},v1=00ff,v0=abc,v1={v1}");

        assert!(verify_signature(SECRET, Some(&header), b"{}", NOW));
    }
}
