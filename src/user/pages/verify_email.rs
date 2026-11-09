use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use topcoat::{
    Error, Result,
    context::{Cx, try_request_context},
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        page, path_param, request, route,
    },
    view::{View, ViewExt, view},
};

use super::expired_link::expired_link;
use super::new_password_form::new_password_form;
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
use crate::web::{app_url, db, session};

const PURPOSE: TokenPurpose = TokenPurpose::EmailVerification;

path_param!(token);

pub async fn issue_verification(
    cx: &Cx,
    user_id: i32,
    email: &str,
) -> Result<std::result::Result<(), String>> {
    let tx = db(cx).begin().await?;
    issue_verification_in(cx, tx, user_id, email).await
}

pub async fn issue_verification_in(
    cx: &Cx,
    tx: Transaction<'_, Postgres>,
    user_id: i32,
    email: &str,
) -> Result<std::result::Result<(), String>> {
    email_user_link(cx, tx, user_id, PURPOSE, |token| {
        let verify_url = app_url(&format!("/user/verify/{token}"))?;
        Ok(register_verification_email(email, &verify_url))
    })
    .await
}

struct SetPasswordFailed {
    problem: PasswordProblem,
}

#[page("/user/verify/{token}")]
pub async fn verify_email_page(cx: &Cx) -> Result<impl View> {
    let token_hash = get_token_hash_from_token_string(path_param::<Token>(cx));
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

    let problem = try_request_context::<SetPasswordFailed>(cx).map(|failed| failed.problem);

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

    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return Err(back_to_form().with(SetPasswordFailed { problem }).into());
    }

    let password_hash = hash_password(&input.password).map_err(|e| Error::msg(e.to_string()))?;
    let token_hash = get_token_hash_from_token_string(path_param::<Token>(cx));
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

    Ok(see_other("/account"))
}
