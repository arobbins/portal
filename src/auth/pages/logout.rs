use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        href, route,
    },
};

use crate::auth::pages::login::login_page;
use crate::web::session;

#[route(POST "/logout")]
pub async fn logout(cx: &Cx) -> Result<SeeOther> {
    session(cx)?.flush().await?;

    Ok(see_other(href!(login_page).resolve(cx)))
}
