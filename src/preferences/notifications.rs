use sqlx::PgExecutor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum NotificationTopic {
    SignInAlerts,
}

impl NotificationTopic {
    pub const ALL: [Self; 1] = [Self::SignInAlerts];

    pub fn label(self) -> &'static str {
        match self {
            Self::SignInAlerts => "New sign-in alerts",
        }
    }
}

pub async fn wants(
    user_id: i32,
    topic: NotificationTopic,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let opted_out = super::queries::query_get_opt_out(user_id, topic, db).await?;
    Ok(!opted_out)
}
