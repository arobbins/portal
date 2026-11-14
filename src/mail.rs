use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::header::ACCEPT;
use serde::Serialize;

static NEXT_MAIL_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize)]
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
    Postmark {
        client: reqwest::Client,
        server_token: String,
        base_url: String,
        from: String,
    },
    Log,
    File {
        dir: PathBuf,
    },
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
            Self::File { dir } => {
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
            Self::Postmark {
                client,
                server_token,
                base_url,
                from,
            } => {
                let body = PostmarkEmail {
                    from,
                    to: &email.to,
                    subject: &email.subject,
                    text_body: &email.text,
                    html_body: &email.html,
                    message_stream: "outbound",
                };

                let response = client
                    .post(format!("{base_url}/email"))
                    .header("X-Postmark-Server-Token", server_token)
                    .header(ACCEPT, "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(MailError::new)?;

                let status = response.status();
                let response_body = response.text().await.map_err(MailError::new)?;

                if !status.is_success() {
                    return Err(MailError::new(format!(
                        "Postmark returned {status}: {response_body}"
                    )));
                }

                Ok(())
            }
        }
    }
}
