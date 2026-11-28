use serde::{Deserialize, Serialize};

use super::Email;
use crate::env::App;
use crate::jobs::{Job, JobError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendEmail(pub Email);

impl Job for SendEmail {
    const KIND: &'static str = "send_email";

    async fn run(self, app: &App) -> Result<(), JobError> {
        app.config.mailer.send(&self.0).await.map_err(JobError::new)
    }
}
