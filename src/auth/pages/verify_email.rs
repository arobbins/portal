use topcoat::{
    Result,
    context::{Cx, try_context},
    metadata::Metadata,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, page, path_param, route,
    },
    view::{View, ViewExt, view},
};

use crate::app::page_title;
use crate::auth::actions::{NewPassword, link_is_valid, set_password_from_link};
use crate::auth::links::TokenPurpose;
use crate::auth::pages::expired_link::expired_link;
use crate::auth::pages::new_password_form::new_password_form;
use crate::auth::pages::register::register_page;
use crate::auth::pages::reset_password::{LinkPasswordFailed, link_password_failed};
use crate::auth::session::sign_in;
use crate::flash::{Flash, set_flash};
use crate::layout::page_heading::page_heading;
use crate::passkeys::pages::passkey_setup::after_password_sign_in;
use crate::web::session;

const PURPOSE: TokenPurpose = TokenPurpose::EmailVerification;

path_param!(token);

#[page(
    "/verify-email/{token}",
    with(Metadata::new().title(page_title("Set your password")))
)]
pub async fn verify_email_page(cx: &Cx) -> Result<impl View> {
    if !link_is_valid(cx, PURPOSE, token(cx)).await? {
        return Ok(view! {
            expired_link(
                <p>
                    "This confirmation link is invalid or has already been used. Links work once and expire after "
                    (PURPOSE.lifetime_text())
                    "."
                </p>
                <p>
                    "To get a new one, "
                    <a href=(href!(register_page).resolve(cx))>"register again"</a>
                    " with the same email."
                </p>
            )
        }
        .boxed());
    }

    let problem = try_context::<LinkPasswordFailed>(cx).map(|failed| failed.problem);

    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "Set your password")
            <p class="p-auth__intro">
                "Your email is almost confirmed. Choose a password to finish setting up your account."
            </p>
            new_password_form(submit_label: "Set password", problem: problem)
        </section>
    }
    .boxed())
}

#[route(POST "/verify-email/{token}")]
pub async fn set_password_submit(cx: &Cx, Form(input): Form<NewPassword>) -> Result<SeeOther> {
    match set_password_from_link(cx, PURPOSE, token(cx), &input).await? {
        Ok(user) => {
            let session = session(cx)?;
            sign_in(&session, &user).await?;
            set_flash(&session, Flash::EmailConfirmed).await?;
            Ok(see_other(
                after_password_sign_in(cx, &session, &user).await?,
            ))
        }
        Err(problem) => Err(link_password_failed(cx, problem)),
    }
}
