pub mod access;
pub mod actions;
pub mod authorize;
mod client;
pub mod emails;
pub mod export;
pub mod grant;
pub mod lifecycle;
mod metadata_document;
pub mod pages;
mod queries;
mod redirect;
mod response;
pub mod revoke;
pub mod routes;
pub mod secrets;
#[cfg(test)]
pub mod testing;

pub(crate) const MAX_CONNECTIONS_PER_USER: i64 = 50;

pub fn app_label(name: &str, return_host: &str) -> String {
    format!("{name} ({return_host})")
}
