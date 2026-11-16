mod account;
mod action;
mod api;
mod api_tokens;
mod app;
mod auth;
mod cleanup;
mod components;
mod config;
mod env;
mod flash;
mod health;
mod jobs;
mod layout;
mod mail;
mod mcp;
mod pages;
mod passkeys;
mod permission;
mod profile;
mod rate_limit;
mod routes;
mod secret;
mod seed;
mod server;
mod sql_console;
mod syntax;
mod theme;
mod user;
mod utils;
mod web;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

enum Command {
    Serve,
    Work,
    Seed,
    Sql,
}

impl Command {
    fn from_args() -> Self {
        match std::env::args().nth(1).as_deref() {
            Some("work") => Self::Work,
            Some("seed") => Self::Seed,
            Some("sql") => Self::Sql,
            _ => Self::Serve,
        }
    }
}

#[tokio::main]
async fn main() {
    init_tracing();
    dotenvy::dotenv().ok();

    match Command::from_args() {
        Command::Serve => server::serve().await,
        Command::Work => server::work().await,
        Command::Seed => seed::run(&server::database_url()).await,
        Command::Sql => sql_console::run(&server::database_url(), std::env::args().nth(2)).await,
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("{}=info,tower_http=info", env!("CARGO_CRATE_NAME")).into());
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}
