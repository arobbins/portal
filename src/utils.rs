const MAX_EMAIL_LENGTH: usize = 254;

pub fn checkbox<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Checked {
        Flag(bool),
        Text(String),
    }

    Ok(
        match <Checked as serde::Deserialize>::deserialize(deserializer)? {
            Checked::Flag(flag) => flag,
            Checked::Text(text) => matches!(text.as_str(), "on" | "true"),
        },
    )
}

pub fn parse_email(input: &str) -> Option<String> {
    let email = input.trim();
    let is_plausible = email.len() <= MAX_EMAIL_LENGTH
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty());

    is_plausible.then(|| email.to_lowercase())
}

pub fn count(n: i64, one: &str, many: &str) -> String {
    let noun = if n == 1 { one } else { many };
    format!("{n} {noun}")
}

pub fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    #[test]
    fn counts_say_one_thing_or_several_things() {
        assert_eq!(super::count(1, "device", "devices"), "1 device");
        assert_eq!(super::count(0, "device", "devices"), "0 devices");
        assert_eq!(super::count(2, "passkey", "passkeys"), "2 passkeys");
    }

    use super::*;

    #[test]
    fn normalizes_a_valid_email() {
        assert_eq!(
            parse_email("  Ada@Example.TEST "),
            Some("ada@example.test".to_string())
        );
    }

    #[test]
    fn rejects_missing_parts_and_oversized_input() {
        assert_eq!(parse_email(""), None);
        assert_eq!(parse_email("no-at-sign"), None);
        assert_eq!(parse_email("@example.test"), None);
        assert_eq!(parse_email("ada@"), None);
        assert_eq!(
            parse_email(&format!("{}@example.test", "a".repeat(250))),
            None
        );
    }

    #[test]
    fn html_is_escaped_for_emails() {
        assert_eq!(
            escape_html("<a href=\"x\">Tom & 'Jerry'</a>"),
            "&lt;a href=&quot;x&quot;&gt;Tom &amp; &#39;Jerry&#39;&lt;/a&gt;"
        );
    }
}
