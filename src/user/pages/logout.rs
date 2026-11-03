use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        route,
    },
};

use crate::web::session;

#[route(POST "/logout")]
pub async fn logout(cx: &Cx) -> Result<SeeOther> {
    session(cx)?.flush().await?;

    Ok(see_other("/login"))
}
