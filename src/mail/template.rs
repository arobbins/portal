use super::Email;
use crate::utils::escape_html;

pub enum Part<'a> {
    Text(&'a str),
    Link { text: &'a str, url: &'a str },
}

pub struct EmailTemplate<'a> {
    subject: &'a str,
    paragraphs: Vec<Vec<Part<'a>>>,
}

impl<'a> EmailTemplate<'a> {
    pub fn new(subject: &'a str) -> Self {
        Self {
            subject,
            paragraphs: Vec::new(),
        }
    }

    pub fn paragraph(mut self, parts: Vec<Part<'a>>) -> Self {
        self.paragraphs.push(parts);
        self
    }

    pub fn text(self, text: &'a str) -> Self {
        self.paragraph(vec![Part::Text(text)])
    }

    pub fn to(&self, recipient: &str) -> Email {
        Email {
            to: recipient.to_string(),
            subject: self.subject.to_string(),
            text: self.text_body(),
            html: self.html_body(),
        }
    }

    fn text_body(&self) -> String {
        self.paragraphs
            .iter()
            .map(|parts| parts.iter().map(render_text).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    fn html_body(&self) -> String {
        self.paragraphs
            .iter()
            .map(|parts| {
                format!(
                    "<p>{}</p>",
                    parts.iter().map(render_html).collect::<String>()
                )
            })
            .collect()
    }
}

fn render_text(part: &Part<'_>) -> String {
    match part {
        Part::Text(text) => (*text).to_string(),
        Part::Link { text, url } => format!("{text}: {url}"),
    }
}

fn render_html(part: &Part<'_>) -> String {
    match part {
        Part::Text(text) => escape_html(text),
        Part::Link { text, url } => {
            format!("<a href=\"{}\">{}</a>", escape_html(url), escape_html(text))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_template_renders_text_and_escaped_html() {
        let email = EmailTemplate::new("Hello")
            .paragraph(vec![
                Part::Text("Token <b>x</b> added. "),
                Part::Link {
                    text: "Review it",
                    url: "https://example.test/a?b=1&c=2",
                },
            ])
            .text("Thanks.")
            .to("ada@example.test");

        assert_eq!(email.subject, "Hello");
        assert_eq!(
            email.text,
            "Token <b>x</b> added. Review it: https://example.test/a?b=1&c=2\n\nThanks."
        );
        assert_eq!(
            email.html,
            "<p>Token &lt;b&gt;x&lt;/b&gt; added. <a href=\"https://example.test/a?b=1&amp;c=2\">Review it</a></p><p>Thanks.</p>"
        );
    }
}
