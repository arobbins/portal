use base64::Engine;
use qrcode::QrCode;
use qrcode::render::svg;
use totp_rs::{Algorithm, Builder, Secret, Totp};

use crate::app::SITE_NAME;
use crate::secret::random_base62;

pub const RECOVERY_CODE_COUNT: usize = 10;
const STEP_SECONDS: u64 = 30;
const DIGITS: u8 = 6;
const SECRET_LENGTH: usize = 32;
const VERIFY_ACCOUNT: &str = "account";

pub fn new_secret() -> String {
    Secret::generate().to_base32()
}

pub fn is_secret(secret: &str) -> bool {
    secret.len() == SECRET_LENGTH
        && secret
            .chars()
            .all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c))
}

fn totp(secret: &str, account: &str) -> Option<Totp> {
    if !is_secret(secret) {
        return None;
    }
    Builder::new()
        .with_algorithm(Algorithm::SHA1)
        .with_digits(DIGITS)
        .with_secret(Secret::try_from_base32(secret).ok()?)
        .with_skew(1)
        .with_step_duration(STEP_SECONDS)
        .with_account_name(account)
        .with_issuer(Some(SITE_NAME))
        .build()
        .ok()
}

pub fn setup_url(secret: &str, account: &str) -> Option<String> {
    totp(secret, account)?.to_url().ok()
}

pub fn qr_code_image(url: &str) -> Option<String> {
    let code = QrCode::new(url.as_bytes()).ok()?;
    let image = code
        .render::<svg::Color<'_>>()
        .min_dimensions(200, 200)
        .quiet_zone(true)
        .build();
    let encoded = base64::engine::general_purpose::STANDARD.encode(image);
    Some(format!("data:image/svg+xml;base64,{encoded}"))
}

pub fn matching_step(secret: &str, code: &str, now: i64) -> Option<i64> {
    let code: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    if code.len() != usize::from(DIGITS) || !code.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let step = totp(secret, VERIFY_ACCOUNT)?.check(&code, u64::try_from(now).ok()?)?;
    i64::try_from(step).ok()
}

#[cfg(test)]
pub fn code_at(secret: &str, now: i64) -> String {
    totp(secret, VERIFY_ACCOUNT)
        .expect("a valid secret")
        .generate(now as u64)
        .to_string()
}

pub fn new_recovery_codes() -> Vec<String> {
    (0..RECOVERY_CODE_COUNT)
        .map(|_| {
            let raw = random_base62(10).to_lowercase();
            format!("{}-{}", &raw[..5], &raw[5..])
        })
        .collect()
}

pub fn normalize_recovery_code(input: &str) -> String {
    input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_secret_is_long_enough_and_well_formed() {
        let secret = new_secret();
        assert!(is_secret(&secret), "{secret}");
        assert_ne!(secret, new_secret());
    }

    #[test]
    fn a_code_matches_its_step_and_one_step_either_side() {
        let secret = new_secret();
        let totp = totp(&secret, VERIFY_ACCOUNT).unwrap();
        let now = 1_800_000_000;
        let code = totp.generate(now as u64).to_string();

        assert_eq!(matching_step(&secret, &code, now), Some(now / 30));
        assert_eq!(matching_step(&secret, &code, now + 30), Some(now / 30));
        assert_eq!(matching_step(&secret, &code, now + 90), None);
        assert_eq!(matching_step(&secret, "12345", now), None);
        assert_eq!(matching_step(&secret, "abcdef", now), None);
    }

    #[test]
    fn the_setup_url_names_the_app_and_the_account() {
        let url = setup_url(&new_secret(), "ada@example.test").unwrap();

        assert!(url.starts_with("otpauth://totp/"), "{url}");
        assert!(url.contains("issuer=Webserver"), "{url}");
        assert!(
            qr_code_image(&url)
                .unwrap()
                .starts_with("data:image/svg+xml;base64,")
        );
    }

    #[test]
    fn recovery_codes_are_unique_and_forgiving_to_type() {
        let codes = new_recovery_codes();
        let mut unique = codes.clone();
        unique.sort();
        unique.dedup();

        assert_eq!(unique.len(), RECOVERY_CODE_COUNT);
        assert_eq!(normalize_recovery_code(" AbCde-12345 "), "abcde12345");
    }
}
