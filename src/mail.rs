mod job;
mod template;

pub use job::SendEmail;
pub use template::{EmailTemplate, Part};

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::header::ACCEPT;
use serde::{Deserialize, Serialize};

static NEXT_MAIL_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailError(String);

impl MailError {
    fn new(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

impl std::fmt::Display for MailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MailError {}

#[derive(Debug, Clone)]
pub enum Mailer {
    Postmark(Postmark),
    Log,
    File { dir: PathBuf },
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct PostmarkEmail<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text_body: &'a str,
    html_body: &'a str,
    message_stream: &'a str,
}

impl Mailer {
    pub async fn send(&self, email: &Email) -> Result<(), MailError> {
        match self {
            Self::Log => {
                tracing::info!(to = %email.to, subject = %email.subject, "email (not sent):\n{}", email.text);
                Ok(())
            }
            Self::File { dir } => write_to_file(dir, email).await,
            Self::Postmark(postmark) => postmark.send(email).await,
        }
    }
}

async fn write_to_file(dir: &PathBuf, email: &Email) -> Result<(), MailError> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(MailError::new)?
        .as_nanos();
    let sequence = NEXT_MAIL_FILE.fetch_add(1, Ordering::Relaxed);
    let json = serde_json::to_vec_pretty(email).map_err(MailError::new)?;

    tokio::fs::create_dir_all(dir)
        .await
        .map_err(MailError::new)?;
    tokio::fs::write(dir.join(format!("{nanos}-{sequence:08}.json")), json)
        .await
        .map_err(MailError::new)
}

#[derive(Debug, Clone)]
pub struct Postmark {
    pub client: reqwest::Client,
    pub server_token: String,
    pub base_url: String,
    pub from: String,
}

impl Postmark {
    async fn send(&self, email: &Email) -> Result<(), MailError> {
        let postmark = self;
        let body = PostmarkEmail {
            from: &postmark.from,
            to: &email.to,
            subject: &email.subject,
            text_body: &email.text,
            html_body: &email.html,
            message_stream: "outbound",
        };
        let response = postmark
            .client
            .post(format!("{}/email", postmark.base_url))
            .header("X-Postmark-Server-Token", &postmark.server_token)
            .header(ACCEPT, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(MailError::new)?;

        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        let response_body = response.text().await.map_err(MailError::new)?;
        Err(MailError::new(format!(
            "Postmark returned {status}: {response_body}"
        )))
    }
}
