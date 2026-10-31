pub mod api;
pub mod model;
pub mod pages;

#[cfg(feature = "ssr")]
pub mod auth;
#[cfg(feature = "ssr")]
pub mod controller;
#[cfg(feature = "ssr")]
pub mod queries;
#[cfg(feature = "ssr")]
pub mod service;
