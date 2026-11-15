use topcoat::{Result, context::Cx, router::error::see_other};

use super::caller::Caller;
use crate::routes::LOGIN_PATH;
use crate::web::globals;

pub fn browser_caller(cx: &Cx) -> Option<Caller> {
    globals(cx).current_user.map(Caller::browser)
}

pub fn require_browser_caller(cx: &Cx) -> Result<Caller> {
    browser_caller(cx).ok_or_else(|| see_other(LOGIN_PATH).into())
}
