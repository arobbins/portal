use serde::Deserialize;
use sqlx::PgPool;
use topcoat::{
    Error, Result,
    context::{Cx, try_context},
    metadata::Metadata,
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other, too_many_requests},
        page, path_param,
        request::{self, client_ip},
        route,
    },
    view::{View, ViewExt, view},
};

use super::expired_link::expired_link;
use super::new_password_form::new_password_form;
use super::passkey_setup::after_password_sign_in;
use crate::app::page_title;
use crate::config::Config;
use crate::flash::{Flash, set_flash};
use crate::layout::page_heading::page_heading;
use crate::user::auth::{hash_password, sign_in};
use crate::user::links::email_user_link;
use crate::user::password::{PasswordProblem, check_new_password};
use crate::user::queries::{
    query_consume_user_token, query_delete_user_tokens, query_find_valid_user_token,
    query_set_password_and_verify_user,
};
use crate::user::service::register_verification_email;
use crate::user::tokens::TokenPurpose;
use crate::utils::get_token_hash_from_token_string;
use crate::web::{db, rate_limits, session};

const RETRY_AFTER_SECS: u64 = 10;
const PURPOSE: TokenPurpose = TokenPurpose::EmailVerification;

path_param!(token);

pub async fn issue_verification(
    db: &PgPool,
    config: &Config,
    user_id: i32,
    email: &str,
) -> std::result::Result<std::result::Result<(), String>, sqlx::Error> {
    email_user_link(db, &config.mailer, user_id, PURPOSE, |token| {
        register_verification_email(email, &config.url(&format!("/user/verify/{token}")))
    })
    .await
}

struct SetPasswordFailed {
    problem: PasswordProblem,
}

#[page(
    "/user/verify/{token}",
    with(Metadata::new().title(page_title("Set your password")))
)]
pub async fn verify_email_page(cx: &Cx) -> Result<impl View> {
    let token_hash = get_token_hash_from_token_string(token(cx));
    let is_valid = query_find_valid_user_token(PURPOSE, &token_hash, db(cx))
        .await?
        .is_some();

    if !is_valid {
        return Ok(view! {
            expired_link(
                <p>
                    "This confirmation link is invalid or has already been used. Links work once and expire after 24 hours."
                </p>
                <p>
                    "To get a new one, "
                    <a href="/register">"register again"</a>
                    " with the same email."
                </p>
            )
        }
        .boxed());
    }

    let problem = try_context::<SetPasswordFailed>(cx).map(|failed| failed.problem);

    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "Set your password")
            <p>
                "Your email is almost confirmed. Choose a password to finish setting up your account."
            </p>
            new_password_form(submit_label: "Set password", problem: problem)
        </section>
    }
    .boxed())
}

#[derive(Deserialize)]
pub struct SetPasswordInput {
    password: String,
    confirm_password: String,
}

#[route(POST "/user/verify/{token}")]
pub async fn set_password_submit(cx: &Cx, Form(input): Form<SetPasswordInput>) -> Result<SeeOther> {
    let back_to_form = || rewrite(request::uri(cx).path(), Body::empty()).method(Method::GET);

    if !rate_limits(cx).allow_password_submission(client_ip(cx)) {
        return Err(too_many_requests(RETRY_AFTER_SECS).into());
    }

    let token_hash = get_token_hash_from_token_string(token(cx));
    if query_find_valid_user_token(PURPOSE, &token_hash, db(cx))
        .await?
        .is_none()
    {
        return Err(back_to_form().into());
    }

    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return Err(back_to_form().with(SetPasswordFailed { problem }).into());
    }

    let password_hash = hash_password(&input.password).map_err(|e| Error::msg(e.to_string()))?;
    let mut tx = db(cx).begin().await?;

    let Some(user_id) = query_consume_user_token(PURPOSE, &token_hash, &mut *tx).await? else {
        return Err(back_to_form().into());
    };

    query_delete_user_tokens(user_id, PURPOSE, &mut *tx).await?;
    let user = query_set_password_and_verify_user(user_id, &password_hash, &mut *tx).await?;
    tx.commit().await?;

    let session = session(cx)?;
    sign_in(&session, &user).await?;
    set_flash(&session, Flash::EmailConfirmed).await?;

    Ok(see_other(
        after_password_sign_in(cx, &session, &user).await?,
    ))
}
