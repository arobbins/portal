const MAX_EMAIL_LENGTH: usize = 254;

pub fn parse_email(input: &str) -> Option<String> {
    let email = input.trim();
    let is_plausible = email.len() <= MAX_EMAIL_LENGTH
        && email
            .split_once('@')
            .is_some_and(|(local, domain)| !local.is_empty() && !domain.is_empty());

    is_plausible.then(|| email.to_lowercase())
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

pub fn format_date(date: time::OffsetDateTime) -> String {
    let month = date.month().to_string();
    format!("{} {}, {}", &month[..3], date.day(), date.year())
}

#[cfg(test)]
mod tests {
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
    fn dates_are_short_and_readable() {
        let date = time::OffsetDateTime::from_unix_timestamp(1_793_700_000).unwrap();

        assert_eq!(format_date(date), "Nov 3, 2026");
    }

    #[test]
    fn html_is_escaped_for_emails() {
        assert_eq!(
            escape_html("<a href=\"x\">Tom & 'Jerry'</a>"),
            "&lt;a href=&quot;x&quot;&gt;Tom &amp; &#39;Jerry&#39;&lt;/a&gt;"
        );
    }
}
