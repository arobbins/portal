use askama::Template;
use axum::{
    extract::{Path, Extension},
    http::HeaderMap,
    response::{IntoResponse, Redirect, Response},
};

use crate::web::{GlobalState, HtmlTemplate};
use crate::user::auth::AuthenticatedUser;
use crate::user::model::User;

pub async fn account_page_template(Path(page_name): Path<String>, AuthenticatedUser(user): AuthenticatedUser, Extension(globals): Extension<GlobalState>, headers: HeaderMap) -> Response {
    if headers.contains_key("HX-Request") {
        return match page_name.as_str() {
            "security" => HtmlTemplate(AccountSecurityPartial { globals, user }).into_response(),
            _ => HtmlTemplate(AccountProfilePartial { globals, user }).into_response(),
        };
    }

    let active_tab = match page_name.as_str() {
        "security" => "security",
        _ => "general",
    };

    HtmlTemplate(AccountTemplate { user, globals, active_tab: active_tab.to_string() }).into_response()
}

pub async fn register_template(user: Option<AuthenticatedUser>, Extension(globals): Extension<GlobalState>) -> Response {

    if user.is_some() {
        return Redirect::to("/account").into_response();
    }

    let template = RegisterTemplate { globals };
    HtmlTemplate(template).into_response()
}

pub async fn login_template(
    user: Option<AuthenticatedUser>,
    Extension(globals): Extension<GlobalState>,
) -> Response {
    if user.is_some() {
        return Redirect::to("/account").into_response();
    }

    let template = LoginTemplate { globals };
    HtmlTemplate(template).into_response()
}

pub async fn account_template(
    AuthenticatedUser(user): AuthenticatedUser,
    Extension(globals): Extension<GlobalState>,
) -> impl IntoResponse {
    let template = AccountTemplate { user, globals, active_tab: "general".to_string() };
    HtmlTemplate(template)
}

#[derive(Template)]
#[template(path = "user/account.html")]
pub struct AccountTemplate {
    user: User,
    globals: GlobalState,
    active_tab: String,
}

#[derive(Template)]
#[template(path = "user/account-profile.html")]
pub struct AccountProfilePartial {
    user: User,
    globals: GlobalState
}

#[derive(Template)]
#[template(path = "user/account-security.html")]
pub struct AccountSecurityPartial {
    user: User,
    globals: GlobalState
}

#[derive(Template)]
#[template(path = "user/register.html")]
pub struct RegisterTemplate {
    globals: GlobalState
 }

#[derive(Template)]
#[template(path = "user/login.html")]
pub struct LoginTemplate { 
    globals: GlobalState
}