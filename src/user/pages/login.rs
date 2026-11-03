use argon2::{Argon2, PasswordVerifier};
use password_hash::PasswordHash;
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_request_context},
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        href, page, route,
    },
    runtime::{Event, signal},
    view::{View, attributes, view},
};

use crate::components::button::{ButtonType, button};
use crate::components::form::{InputType, form, form_field};
use crate::components::notice::{NoticeKind, notice};
use crate::layout::page_heading::page_heading;
use crate::user::auth::{redirect_signed_in_users, sign_in};
use crate::user::queries::query_get_user_by_email;
use crate::utils::normalize_email;
use crate::web::{db, session};

#[derive(Deserialize)]
pub struct LoginInput {
    email: String,
    password: String,
}

struct LoginFailed {
    email: String,
    message: &'static str,
}

#[page("/login")]
pub async fn login_page(cx: &Cx) -> Result<impl View> {
    redirect_signed_in_users(cx)?;

    let failed = try_request_context::<LoginFailed>(cx);
    let email = failed.map(|failed| failed.email.clone());
    let error = failed.map(|failed| failed.message);
    let is_submitting = signal(cx, || false);

    Ok(view! {
        <section class="l-contain l-skinny">
            page_heading(text: "Login")
            form(
                attrs: attributes! {
                    class="form form--skinny"
                    method="post"
                    action=(href!(login_submit))
                    @submit=$(|_e: Event| is_submitting.set(true))
                },
                form_field(
                    label: "Email",
                    id: "email",
                    name: "email",
                    input_type: InputType::Email,
                    placeholder: Some("Enter email"),
                    value: email.as_deref(),
                )
                form_field(
                    label: "Password",
                    id: "password",
                    name: "password",
                    input_type: InputType::Password,
                    placeholder: Some("Enter password"),
                )
                button(
                    button_type: ButtonType::Submit,
                    busy: $(is_submitting.get()),
                    disabled: $(is_submitting.get()),
                    "Login"
                )
                if let Some(error) = error {
                    notice(kind: NoticeKind::Error, (error))
                }
            )
        </section>
    })
}

#[route(POST "/login")]
pub async fn login_submit(cx: &Cx, Form(input): Form<LoginInput>) -> Result<SeeOther> {
    match attempt_login(cx, &input).await? {
        Ok(()) => Ok(see_other("/account")),
        Err(message) => Err(rewrite(href!(login_page).resolve(cx), Body::empty())
            .method(Method::GET)
            .with(LoginFailed { email: input.email, message })
            .into()),
    }
}

async fn attempt_login(cx: &Cx, input: &LoginInput) -> Result<Result<(), &'static str>> {
    const INVALID: &str = "Invalid email or password.";

    let email = normalize_email(&input.email);

    let Some(user) = query_get_user_by_email(&email, db(cx)).await? else {
        return Ok(Err(INVALID));
    };

    let Ok(parsed_hash) = PasswordHash::new(&user.password) else {
        return Ok(Err(INVALID));
    };

    if Argon2::default()
        .verify_password(input.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Ok(Err(INVALID));
    }

    if user.verified_at.is_none() {
        return Ok(Err("Please verify your email before logging in."));
    }

    sign_in(&session(cx)?, &user).await?;

    Ok(Ok(()))
}
