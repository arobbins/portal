#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CodeLanguage {
    #[default]
    Plain,
    Json,
    Shell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Plain,
    Key,
    String,
    Number,
    Literal,
    Punctuation,
    Comment,
    Variable,
    Flag,
    Command,
}

impl TokenKind {
    pub fn class(self) -> &'static str {
        match self {
            Self::Plain => "a-code-block__token",
            Self::Key => "a-code-block__token a-code-block__token--key",
            Self::String => "a-code-block__token a-code-block__token--string",
            Self::Number => "a-code-block__token a-code-block__token--number",
            Self::Literal => "a-code-block__token a-code-block__token--literal",
            Self::Punctuation => "a-code-block__token a-code-block__token--punctuation",
            Self::Comment => "a-code-block__token a-code-block__token--comment",
            Self::Variable => "a-code-block__token a-code-block__token--variable",
            Self::Flag => "a-code-block__token a-code-block__token--flag",
            Self::Command => "a-code-block__token a-code-block__token--command",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
}

pub fn highlight(code: &str, language: CodeLanguage) -> Vec<Vec<Token>> {
    let mut continues = false;
    code.split('\n')
        .map(|line| {
            let tokens = match language {
                CodeLanguage::Plain => vec![token(TokenKind::Plain, line)],
                CodeLanguage::Json => json_line(line),
                CodeLanguage::Shell => shell_line(line, continues),
            };
            continues = line.trim_end().ends_with('\\');
            merge(tokens)
        })
        .collect()
}

fn token(kind: TokenKind, text: &str) -> Token {
    Token {
        kind,
        text: text.to_string(),
    }
}

fn merge(tokens: Vec<Token>) -> Vec<Token> {
    let mut merged: Vec<Token> = Vec::new();
    for next in tokens.into_iter().filter(|token| !token.text.is_empty()) {
        match merged.last_mut() {
            Some(last) if last.kind == next.kind => last.text.push_str(&next.text),
            _ => merged.push(next),
        }
    }
    merged
}

fn quoted_end(chars: &[char], start: usize) -> usize {
    let quote = chars[start];
    let mut index = start + 1;
    while index < chars.len() {
        match chars[index] {
            '\\' if quote == '"' => index += 2,
            current if current == quote => return index + 1,
            _ => index += 1,
        }
    }
    chars.len()
}

fn collect(chars: &[char], start: usize, end: usize) -> String {
    chars[start..end.min(chars.len())].iter().collect()
}

fn json_line(line: &str) -> Vec<Token> {
    let chars: Vec<char> = line.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let current = chars[index];
        let (kind, end) = if current == '"' {
            let end = quoted_end(&chars, index);
            let rest = collect(&chars, end, chars.len());
            let kind = if rest.trim_start().starts_with(':') {
                TokenKind::Key
            } else {
                TokenKind::String
            };
            (kind, end)
        } else if current == '-' || current.is_ascii_digit() {
            let end = scan(&chars, index + 1, |c| {
                c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-')
            });
            (TokenKind::Number, end)
        } else if current.is_ascii_alphabetic() {
            let end = scan(&chars, index, |c| c.is_ascii_alphabetic());
            let word = collect(&chars, index, end);
            let kind = if matches!(word.as_str(), "true" | "false" | "null") {
                TokenKind::Literal
            } else {
                TokenKind::Plain
            };
            (kind, end)
        } else if "{}[],:".contains(current) {
            (TokenKind::Punctuation, index + 1)
        } else {
            (TokenKind::Plain, index + 1)
        };
        tokens.push(Token {
            kind,
            text: collect(&chars, index, end),
        });
        index = end;
    }
    tokens
}

fn scan(chars: &[char], start: usize, keep: impl Fn(char) -> bool) -> usize {
    let mut index = start;
    while index < chars.len() && keep(chars[index]) {
        index += 1;
    }
    index
}

fn shell_line(line: &str, continues: bool) -> Vec<Token> {
    let chars: Vec<char> = line.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut expects_command = !continues;
    while index < chars.len() {
        let current = chars[index];
        let starts_word = index == 0 || chars[index - 1].is_whitespace();
        let (kind, end) = if current.is_whitespace() {
            (TokenKind::Plain, scan(&chars, index, char::is_whitespace))
        } else if current == '#' && starts_word {
            (TokenKind::Comment, chars.len())
        } else if current == '\'' || current == '"' {
            (TokenKind::String, quoted_end(&chars, index))
        } else if current == '$' {
            let end = if chars.get(index + 1) == Some(&'{') {
                scan(&chars, index + 2, |c| c != '}') + 1
            } else {
                scan(&chars, index + 1, |c| c.is_ascii_alphanumeric() || c == '_')
            };
            (TokenKind::Variable, end)
        } else if current == '\\' && index + 1 == chars.len() {
            (TokenKind::Punctuation, index + 1)
        } else if current == '-' && starts_word {
            (TokenKind::Flag, scan(&chars, index, |c| !c.is_whitespace()))
        } else {
            let end = scan(&chars, index, |c| {
                !c.is_whitespace() && !matches!(c, '$' | '\'' | '"')
            });
            let kind = if expects_command && starts_word {
                TokenKind::Command
            } else {
                TokenKind::Plain
            };
            (kind, end)
        };
        if !matches!(kind, TokenKind::Plain) || !current.is_whitespace() {
            expects_command = false;
        }
        tokens.push(Token {
            kind,
            text: collect(&chars, index, end),
        });
        index = end.max(index + 1);
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &[Token]) -> Vec<(TokenKind, &str)> {
        line.iter()
            .map(|token| (token.kind, token.text.as_str()))
            .collect()
    }

    #[test]
    fn json_keys_strings_numbers_and_literals_are_told_apart() {
        let lines = highlight(
            "{\n  \"name\": \"Ada\",\n  \"age\": 36, \"admin\": false\n}",
            CodeLanguage::Json,
        );

        assert_eq!(lines.len(), 4);
        assert_eq!(
            kinds(&lines[1]),
            vec![
                (TokenKind::Plain, "  "),
                (TokenKind::Key, "\"name\""),
                (TokenKind::Punctuation, ":"),
                (TokenKind::Plain, " "),
                (TokenKind::String, "\"Ada\""),
                (TokenKind::Punctuation, ","),
            ]
        );
        assert!(kinds(&lines[2]).contains(&(TokenKind::Number, "36")));
        assert!(kinds(&lines[2]).contains(&(TokenKind::Literal, "false")));
    }

    #[test]
    fn escaped_quotes_stay_inside_json_strings() {
        let lines = highlight("\"say \\\"hi\\\"\"", CodeLanguage::Json);

        assert_eq!(
            kinds(&lines[0]),
            vec![(TokenKind::String, "\"say \\\"hi\\\"\"")]
        );
    }

    #[test]
    fn shell_commands_flags_strings_and_variables_are_told_apart() {
        let lines = highlight(
            "curl https://x.test/me \\\n  -H \"Authorization: Bearer $WSK_TOKEN\" # note",
            CodeLanguage::Shell,
        );

        assert_eq!(lines[0][0], token(TokenKind::Command, "curl"));
        assert_eq!(lines[0].last().unwrap().kind, TokenKind::Punctuation);
        assert!(kinds(&lines[1]).contains(&(TokenKind::Flag, "-H")));
        assert!(
            kinds(&lines[1]).contains(&(TokenKind::String, "\"Authorization: Bearer $WSK_TOKEN\""))
        );
        assert!(kinds(&lines[1]).contains(&(TokenKind::Comment, "# note")));
        assert!(
            kinds(&highlight("echo $HOME", CodeLanguage::Shell)[0])
                .contains(&(TokenKind::Variable, "$HOME"))
        );
    }

    #[test]
    fn a_continued_shell_line_has_no_command() {
        let lines = highlight("curl \\\n  https://x.test", CodeLanguage::Shell);

        assert!(
            lines[1]
                .iter()
                .all(|token| token.kind != TokenKind::Command)
        );
    }

    #[test]
    fn highlighting_never_changes_the_text() {
        for (code, language) in [
            (
                "{\"a\": [1, -2.5e3, null], \"b\": \"x\"}",
                CodeLanguage::Json,
            ),
            (
                "export A=b\n\ncurl -d '{\"x\":1}' $A \\",
                CodeLanguage::Shell,
            ),
            ("plain <b>text</b>", CodeLanguage::Plain),
            ("\"unterminated", CodeLanguage::Json),
            ("echo 'open", CodeLanguage::Shell),
        ] {
            let rebuilt: Vec<String> = highlight(code, language)
                .iter()
                .map(|line| line.iter().map(|token| token.text.as_str()).collect())
                .collect();
            assert_eq!(rebuilt.join("\n"), code);
        }
    }
}
