use sqlx::PgExecutor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
pub enum Newsletter {
    GeneralNewsletter,
    ProductUpdates,
}

impl Newsletter {
    pub const ALL: [Self; 2] = [Self::GeneralNewsletter, Self::ProductUpdates];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::GeneralNewsletter => "general_newsletter",
            Self::ProductUpdates => "product_updates",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::GeneralNewsletter => "General newsletter",
            Self::ProductUpdates => "Product updates",
        }
    }
}

pub async fn subscribed(
    user_id: i32,
    newsletter: Newsletter,
    db: impl PgExecutor<'_>,
) -> Result<bool, sqlx::Error> {
    let opted_out = super::queries::query_get_newsletter_opt_out(user_id, newsletter, db).await?;
    Ok(!opted_out)
}

pub async fn newsletters_of(
    user_id: i32,
    db: &sqlx::PgPool,
) -> Result<Vec<(Newsletter, bool)>, sqlx::Error> {
    let mut choices = Vec::new();
    for newsletter in Newsletter::ALL {
        choices.push((newsletter, subscribed(user_id, newsletter, db).await?));
    }
    Ok(choices)
}
