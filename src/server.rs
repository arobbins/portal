pub mod router;
#[cfg(test)]
mod tests;

use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::{ServiceExt, extract::Request, middleware};
use listenfd::ListenFd;
use sqlx::PgPool;
use tokio::net::TcpListener;
use topcoat::asset::AssetBundle;
use tower::Layer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_sessions::cookie::SameSite;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

use crate::auth::actions::{EmailChangeJob, PasswordResetJob};
use crate::billing::jobs::{CancelStripeSubscription, UpdateStripeCustomerEmail};
use crate::cleanup::CleanupJob;
use crate::config::Config;
use crate::devices::lifecycle::DEVICE_IDLE_LIMIT;
use crate::env::{App, Clock};
use crate::jobs::{Handler, Worker};
use crate::mail::SendEmail;
use crate::passkeys::webauthn::relying_party;
use crate::rate_limit::RateLimits;
use crate::web::run_to_completion;

pub use router::router;

pub async fn serve() {
    let (app, sessions) = start().await;
    worker(&app).spawn(app.config.job_workers);

    let bind_address = app.config.bind_address.clone();
    let session_layer = session_layer(&app.config, sessions);
    let http = router(app, load_assets())
        .layer(session_layer)
        .layer(middleware::from_fn(run_to_completion));
    let http = NormalizePathLayer::trim_trailing_slash().layer(http);

    let listener = listener(&bind_address).await;
    topcoat::dev::notify_ready(listener.local_addr().ok()).await;
    let service = ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(http);
    if let Err(e) = axum::serve(listener, service).await {
        tracing::error!("server stopped: {e}");
    }
}

pub async fn work() {
    let (app, _) = start().await;
    let count = app.config.job_workers.max(1);
    tracing::info!("running {count} job worker(s) without the web server");
    worker(&app).spawn(count);
    std::future::pending::<()>().await;
}

pub fn database_url() -> String {
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

fn job_handlers() -> Vec<Handler> {
    vec![
        Handler::of::<SendEmail>(),
        Handler::of::<PasswordResetJob>(),
        Handler::of::<EmailChangeJob>(),
        Handler::of::<CleanupJob>(),
        Handler::of::<CancelStripeSubscription>(),
        Handler::of::<UpdateStripeCustomerEmail>(),
    ]
}

fn worker(app: &App) -> Worker {
    Worker::new(app.clone(), job_handlers())
}

async fn start() -> (App, PostgresStore) {
    let config = load_config();
    let db = PgPool::connect(&database_url())
        .await
        .expect("could not connect to DATABASE_URL");
    let sessions = PostgresStore::new(db.clone());
    sessions
        .migrate()
        .await
        .expect("could not create the session table");

    (build_app(db, config), sessions)
}

fn load_config() -> Config {
    Config::from_env().unwrap_or_else(|problems| {
        eprintln!("Invalid configuration:\n  - {}", problems.join("\n  - "));
        std::process::exit(1);
    })
}

fn build_app(db: PgPool, config: Config) -> App {
    App {
        limits: Arc::new(RateLimits::new(db.clone(), config.rate_limit_mode)),
        webauthn: Arc::new(relying_party(&config)),
        config: Arc::new(config),
        clock: Clock::System,
        db,
    }
}

fn load_assets() -> AssetBundle {
    AssetBundle::load()
        .expect("missing Topcoat asset bundle: run `topcoat dev` or `topcoat asset bundle`")
}

fn session_layer(config: &Config, sessions: PostgresStore) -> SessionManagerLayer<PostgresStore> {
    SessionManagerLayer::new(sessions)
        .with_secure(config.secure_cookies())
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(DEVICE_IDLE_LIMIT))
}

async fn listener(bind_address: &str) -> TcpListener {
    let inherited = ListenFd::from_env()
        .take_tcp_listener(0)
        .expect("could not read the inherited socket");
    let listener = match inherited {
        Some(listener) => {
            listener
                .set_nonblocking(true)
                .expect("could not configure the inherited socket");
            TcpListener::from_std(listener).expect("could not use the inherited socket")
        }
        None => TcpListener::bind(bind_address)
            .await
            .unwrap_or_else(|e| panic!("could not listen on {bind_address}: {e}")),
    };
    tracing::debug!("listening on {:?}", listener.local_addr());
    listener
}
