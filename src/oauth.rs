pub mod actions;
pub mod authorize;
pub mod client;
pub mod emails;
pub mod grant;
pub mod lifecycle;
mod metadata_document;
pub mod pages;
mod queries;
pub mod redirect;
mod response;
pub mod routes;
pub mod secrets;
#[cfg(test)]
pub mod testing;

pub const MAX_CONNECTIONS_PER_USER: i64 = 50;

pub fn app_label(name: &str, return_host: &str) -> String {
    format!("{name} ({return_host})")
}
