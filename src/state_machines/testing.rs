use std::net::SocketAddr;

use axum::body::Body;
use axum::http::{Request, Response, StatusCode, header};
use time::{Duration, OffsetDateTime};
use topcoat::asset::AssetBundle;
use tower::ServiceExt;
use tower_sessions::{MemoryStore, SessionManagerLayer};

use crate::env::{App, Clock};
use crate::server::router::router;

pub fn assets() -> AssetBundle {
    AssetBundle::load_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/assets"))
        .expect("missing asset bundle: run `topcoat asset bundle` before `cargo test`")
}

pub struct Browser {
    app: App,
    assets: AssetBundle,
    store: MemoryStore,
    pub cookie: Option<String>,
    ip: [u8; 4],
    start: OffsetDateTime,
    pub minutes: i64,
}

impl Browser {
    pub fn new(app: &App, assets: &AssetBundle, ip: [u8; 4]) -> Self {
        Self {
            app: app.clone(),
            assets: assets.clone(),
            store: MemoryStore::default(),
            cookie: None,
            ip,
            start: OffsetDateTime::now_utc(),
            minutes: 0,
        }
    }

    pub fn now(&self) -> OffsetDateTime {
        self.start + Duration::minutes(self.minutes)
    }

    pub async fn sign_in(&mut self, email: &str, password: &str) -> bool {
        let form = format!("email={}&password={password}", email.replace('@', "%40"));
        let response = self.post("/login", &form).await;
        response.status() == StatusCode::SEE_OTHER
            && location(&response).as_deref() != Some("/login")
    }

    pub async fn get(&mut self, path: &str) -> Response<Body> {
        let request = self.request("GET", path).body(Body::empty()).unwrap();
        self.send(request).await
    }

    pub fn session(&self) -> Option<tower_sessions::Session> {
        use std::str::FromStr;

        let id = self.cookie.as_deref()?.strip_prefix("id=")?;
        let id = tower_sessions::session::Id::from_str(id).ok()?;
        let store = std::sync::Arc::new(self.store.clone());
        Some(tower_sessions::Session::new(Some(id), store, None))
    }

    pub async fn session_value(&self, key: &str) -> Option<serde_json::Value> {
        use std::str::FromStr;
        use tower_sessions::session_store::SessionStore;

        let id = self.cookie.as_deref()?.strip_prefix("id=")?;
        let id = tower_sessions::session::Id::from_str(id).ok()?;
        let record = self.store.load(&id).await.ok()??;
        record.data.get(key).cloned()
    }

    pub async fn call_api(
        &mut self,
        method: &str,
        path: &str,
        token: &str,
        json: Option<String>,
    ) -> Response<Body> {
        let request = self
            .request(method, path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(json.map_or_else(Body::empty, Body::from))
            .unwrap();
        self.send(request).await
    }

    pub async fn get_with_token(&mut self, path: &str, token: &str) -> Response<Body> {
        let request = self
            .request("GET", path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        self.send(request).await
    }

    pub async fn post_json(&mut self, path: &str, json: &str) -> Response<Body> {
        let request = self
            .request("POST", path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string()))
            .unwrap();
        self.send(request).await
    }

    pub async fn post(&mut self, path: &str, form: &str) -> Response<Body> {
        let request = self
            .request("POST", path)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(form.to_string()))
            .unwrap();
        self.send(request).await
    }

    fn request(&self, method: &str, path: &str) -> axum::http::request::Builder {
        let origin = self.app.config.origin();
        let builder = Request::builder()
            .method(method)
            .uri(path)
            .header(header::ORIGIN, origin)
            .header(header::HOST, host(origin))
            .extension(axum::extract::ConnectInfo(SocketAddr::from((
                self.ip, 50_000,
            ))));
        match &self.cookie {
            Some(cookie) => builder.header(header::COOKIE, cookie),
            None => builder,
        }
    }

    async fn send(&mut self, request: Request<Body>) -> Response<Body> {
        let app = App {
            clock: Clock::Fixed(self.now()),
            ..self.app.clone()
        };
        let service = router(app, self.assets.clone())
            .layer(SessionManagerLayer::new(self.store.clone()).with_secure(false));
        let response = service.oneshot(request).await.unwrap();
        if let Some(cookie) = response.headers().get(header::SET_COOKIE) {
            let cookie = cookie.to_str().unwrap();
            self.cookie = cookie.split(';').next().map(str::to_string);
        }
        response
    }
}

fn host(origin: &str) -> &str {
    origin.split_once("://").map_or(origin, |(_, host)| host)
}

pub fn location(response: &Response<Body>) -> Option<String> {
    response
        .headers()
        .get(header::LOCATION)
        .map(|value| value.to_str().unwrap().to_string())
}

pub async fn admin_caller(email: &str, db: &sqlx::PgPool) -> crate::action::Caller {
    let user = crate::user::testing::active_user(email, db).await;
    sqlx::query!(
        "UPDATE users SET role = 'admin', admin_granted_at = now() WHERE id = $1",
        user.id
    )
    .execute(db)
    .await
    .unwrap();
    let user = crate::user::queries::query_get_user_by_id(user.id, db)
        .await
        .unwrap()
        .unwrap();
    crate::action::Caller::browser(std::sync::Arc::new(user))
}

pub async fn latest_link(
    purpose: crate::auth::links::TokenPurpose,
    to: &str,
    db: &sqlx::PgPool,
) -> Option<String> {
    let prefix = purpose.path("");
    let bodies = sqlx::query_scalar!(
        "SELECT payload->>'text' FROM jobs WHERE kind = 'send_email' AND payload->>'to' = $1 ORDER BY id DESC",
        to,
    )
    .fetch_all(db)
    .await
    .unwrap();
    bodies.into_iter().flatten().find_map(|body| {
        let start = body.find(&prefix)? + prefix.len();
        let token: String = body[start..]
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        (!token.is_empty()).then_some(token)
    })
}
