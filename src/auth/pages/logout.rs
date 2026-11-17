use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        href, route,
    },
};

use crate::auth::pages::login::login_page;
use crate::auth::session::sign_out;
use crate::web::{app, session};

#[route(POST "/logout")]
pub async fn logout(cx: &Cx) -> Result<SeeOther> {
    sign_out(app(cx), &session(cx)?).await?;

    Ok(see_other(href!(login_page).resolve(cx)))
}
