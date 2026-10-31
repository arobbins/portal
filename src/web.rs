use leptos::prelude::ServerFnError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrentUser {
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct GlobalState {
    pub year: i32,
    pub current_user: Option<CurrentUser>,
    pub current_slug: String,
}

pub fn use_globals() -> GlobalState {
    #[cfg(feature = "ssr")]
    {
        use axum::http::request::Parts;
        use leptos::prelude::use_context;

        if let Some(globals) = use_context::<Parts>()
            .and_then(|parts| parts.extensions.get::<GlobalState>().cloned())
        {
            return globals;
        }
    }

    GlobalState::default()
}

pub fn error_message(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError(message) => message.clone(),
        _ => "Something went wrong. Please try again.".to_string(),
    }
}

#[cfg(feature = "ssr")]
pub use server::*;

#[cfg(feature = "ssr")]
mod server {
    use super::{CurrentUser, GlobalState};
    use crate::utils::create_body_page_class;

    use axum::{
        extract::{FromRef, Request},
        http::Uri,
        middleware::Next,
        response::Response,
    };
    use chrono::Datelike;
    use leptos::config::LeptosOptions;
    use sqlx::PgPool;
    use tower_sessions::Session;

    #[derive(Clone, FromRef)]
    pub struct AppState {
        pub db: PgPool,
        pub leptos_options: LeptosOptions,
    }

    pub async fn inject_globals(session: Session, uri: Uri, mut req: Request, next: Next) -> Response {
        let current_user = session
            .get::<CurrentUser>("current_user")
            .await
            .unwrap_or(None);

        let current_path = uri.path().trim_start_matches('/').to_string();

        req.extensions_mut().insert(GlobalState {
            year: chrono::Local::now().year(),
            current_user,
            current_slug: create_body_page_class(current_path),
        });

        next.run(req).await
    }
}
