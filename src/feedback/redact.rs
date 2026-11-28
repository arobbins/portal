use crate::api_tokens::token::TOKEN_PREFIX;

const REDACTED: &str = "[redacted]";
const BEARER: &str = "bearer";

pub fn redact_secrets(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut hide_next = false;
    for piece in text.split_inclusive(char::is_whitespace) {
        let word = piece.trim_end();
        let gap = &piece[word.len()..];
        if !word.is_empty() {
            let shown = if hide_next {
                REDACTED.to_string()
            } else {
                redact_tokens(word)
            };
            out.push_str(&shown);
            hide_next = !hide_next && is_bearer(word);
        }
        out.push_str(gap);
    }
    out
}

fn is_bearer(word: &str) -> bool {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .eq_ignore_ascii_case(BEARER)
}

fn redact_tokens(word: &str) -> String {
    let mut out = String::new();
    let mut rest = word;
    while let Some(start) = rest.find(TOKEN_PREFIX) {
        out.push_str(&rest[..start]);
        let after = &rest[start + TOKEN_PREFIX.len()..];
        let end = after
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(after.len());
        out.push_str(REDACTED);
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_and_bearer_values_are_removed() {
        assert_eq!(
            redact_secrets("Sent Authorization: Bearer abc.def and wsk_live_123, got 401"),
            "Sent Authorization: Bearer [redacted] and [redacted], got 401"
        );
        assert_eq!(
            redact_secrets("token=\"wsk_9xY\"\nnext"),
            "token=\"[redacted]\"\nnext"
        );
    }

    #[test]
    fn ordinary_text_is_left_alone() {
        let text = "The name field rejects  emoji.\nExpected 200.";

        assert_eq!(redact_secrets(text), text);
    }

    #[test]
    fn a_bearer_value_on_the_next_line_or_missing_is_handled() {
        assert_eq!(
            redact_secrets("Bearer\nabc rest"),
            "Bearer\n[redacted] rest"
        );
        assert_eq!(redact_secrets("ends with Bearer"), "ends with Bearer");
    }
}
