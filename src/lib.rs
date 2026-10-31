#![recursion_limit = "256"]

pub mod app;
pub mod components;
pub mod layout;
pub mod pages;
pub mod user;
pub mod web;

#[cfg(feature = "ssr")]
pub mod utils;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_islands();
}
