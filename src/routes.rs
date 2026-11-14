use topcoat::{context::Cx, router::href};

use crate::account::pages::account::account_page;
use crate::auth::pages::login::login_page;
use crate::auth::pages::logout::logout;
use crate::auth::pages::register::register_page;
use crate::pages::api_docs::api_docs_page;

pub fn login_url(cx: &Cx) -> String {
    href!(login_page).resolve(cx)
}

pub fn logout_url(cx: &Cx) -> String {
    href!(logout).resolve(cx)
}

pub fn register_url(cx: &Cx) -> String {
    href!(register_page).resolve(cx)
}

pub fn account_url(cx: &Cx) -> String {
    href!(account_page).resolve(cx)
}

pub fn api_docs_url(cx: &Cx) -> String {
    href!(api_docs_page).resolve(cx)
}
