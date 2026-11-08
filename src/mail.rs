use std::env;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::header::ACCEPT;
use serde::Serialize;

const FROM_ADDRESS: &str = "hello@andrewrobbins.dev";

#[derive(Serialize)]
pub struct Email {
    pub to: String,
    pub subject: String,
    pub text: String,
    pub html: String,
}

#[derive(Debug, Clone)]
pub enum Mailer {
    Postmark {
        server_token: String,
        base_url: String,
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
    pub fn from_env() -> Self {
        match env::var("MAIL_TRANSPORT").as_deref() {
            Ok("log") => Self::Log,
            Ok("file") => Self::File {
                dir: env::var("MAIL_DIR")
                    .expect("MAIL_DIR must be set when MAIL_TRANSPORT=file")
                    .into(),
            },
            _ => Self::Postmark {
                server_token: env::var("POSTMARK_SERVER_TOKEN")
                    .expect("POSTMARK_SERVER_TOKEN must be set, or set MAIL_TRANSPORT=log"),
                base_url: env::var("POSTMARK_BASE_URL")
                    .expect("POSTMARK_BASE_URL must be set, or set MAIL_TRANSPORT=log"),
            },
        }
    }

    pub async fn send(&self, email: &Email) -> Result<(), String> {
        match self {
            Self::Log => {
                tracing::info!(to = %email.to, subject = %email.subject, "email (not sent):\n{}", email.text);
                Ok(())
            }
            Self::File { dir } => {
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| e.to_string())?
                    .as_nanos();
                let json = serde_json::to_vec_pretty(email).map_err(|e| e.to_string())?;

                tokio::fs::create_dir_all(dir)
                    .await
                    .map_err(|e| e.to_string())?;
                tokio::fs::write(dir.join(format!("{nanos}.json")), json)
                    .await
                    .map_err(|e| e.to_string())
            }
            Self::Postmark {
                server_token,
                base_url,
            } => {
                let body = PostmarkEmail {
                    from: FROM_ADDRESS,
                    to: &email.to,
                    subject: &email.subject,
                    text_body: &email.text,
                    html_body: &email.html,
                    message_stream: "outbound",
                };

                let response = reqwest::Client::new()
                    .post(format!("{base_url}/email"))
                    .header("X-Postmark-Server-Token", server_token)
                    .header(ACCEPT, "application/json")
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| e.to_string())?;

                let status = response.status();
                let response_body = response.text().await.map_err(|e| e.to_string())?;

                if !status.is_success() {
                    return Err(format!("Postmark returned {status}: {response_body}"));
                }

                Ok(())
            }
        }
    }
}
