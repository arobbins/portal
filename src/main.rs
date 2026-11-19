mod account;
mod action;
mod activity;
mod admin;
mod api;
mod api_tokens;
mod app;
mod auth;
mod cleanup;
mod components;
mod config;
mod data_export;
mod devices;
mod env;
mod feedback;
mod flash;
mod health;
mod jobs;
mod layout;
mod listing;
mod mail;
mod mcp;
mod pages;
mod passkeys;
mod permission;
mod preferences;
mod profile;
mod rate_limit;
mod routes;
mod secret;
mod seed;
mod server;
mod sql_console;
#[cfg(test)]
mod state_machines;
mod syntax;
mod theme;
mod time_zone;
mod two_factor;
mod user;
mod utils;
mod web;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

enum Command {
    Serve,
    Work,
    Seed,
    Sql,
    Admin,
}

impl Command {
    fn from_args() -> Self {
        match std::env::args().nth(1).as_deref() {
            Some("work") => Self::Work,
            Some("seed") => Self::Seed,
            Some("sql") => Self::Sql,
            Some("admin") => Self::Admin,
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
        Command::Admin => {
            let args: Vec<String> = std::env::args().skip(2).collect();
            admin::console::run(&server::database_url(), &args).await
        }
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
