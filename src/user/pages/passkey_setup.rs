use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{
        error::{SeeOther, redirect, see_other},
        href, page, route,
    },
    runtime::signal,
    view::{View, attributes, view},
};
use tower_sessions::Session;

use super::account_passkeys::ADD_ERROR_ID;
use crate::app::page_title;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::form::{FormVariant, form, form_error};
use crate::components::icon::{IconName, icon};
use crate::flash::{Flash, flash_toasts, take_flash, toast_name};
use crate::layout::page_heading::page_heading;
use crate::user::auth::{
    current_user, grant_passkey_prompt, passkey_prompt_granted, require_user, revoke_passkey_prompt,
};
use crate::user::model::User;
use crate::user::queries::{
    query_decline_passkey_prompt, query_passkey_prompt_due, query_snooze_passkey_prompt,
};
use crate::web::{db, session};

const ACCOUNT_URL: &str = "/account";
const PASSKEYS_URL: &str = "/account/security#passkeys";
const SNOOZE_DAYS: i32 = 30;
const FLASHES: &[Flash] = &[Flash::EmailConfirmed];

pub async fn after_password_sign_in(cx: &Cx, session: &Session, user: &User) -> Result<String> {
    if query_passkey_prompt_due(user.id, db(cx)).await? {
        grant_passkey_prompt(session, user).await?;
        Ok(href!(passkey_setup_page).resolve(cx))
    } else {
        Ok(ACCOUNT_URL.to_string())
    }
}

#[page("/passkey-setup", with(Metadata::new().title(page_title("Add a passkey"))))]
pub async fn passkey_setup_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;
    let prompt_ready = passkey_prompt_granted(&session(cx)?, &user).await?
        && query_passkey_prompt_due(user.id, db(cx)).await?;
    if !prompt_ready {
        return Err(redirect(ACCOUNT_URL).into());
    }

    let flash = take_flash(cx, FLASHES).await?;
    let toast = signal(cx, || toast_name(flash).to_string());
    let is_skipping = signal(cx, || false);
    let is_declining = signal(cx, || false);

    Ok(view! {
        <section class="l-contain l-skinny passkey-setup">
            <div class="passkey-setup__badge">icon(name: IconName::Passkey)</div>
            page_heading(text: "Sign in faster next time")
            <p class="passkey-setup__intro">
                "Save a passkey and sign in with your fingerprint, face or screen lock instead of typing your password. Passkeys can't be phished or reused on other sites."
            </p>
            form(
                variant: FormVariant::Page,
                attrs: attributes! {
                    class="passkey-setup__add"
                    data-passkey-register=""
                    data-passkey-only=""
                    data-passkey-done=(PASSKEYS_URL)
                },
                form_error(id: Some(ADD_ERROR_ID), hidden: true, "")
                button(button_type: ButtonType::Submit, "Add a passkey")
            )
            <noscript>
                <p class="passkey-setup__intro">"Adding a passkey needs JavaScript."</p>
            </noscript>
            form(
                submitting: Some(&is_skipping),
                attrs: attributes! {
                    class="passkey-setup__skip"
                    method="post"
                    action=(href!(snooze_passkey_prompt))
                },
                button(
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Submit,
                    busy: $(is_skipping.get()),
                    "Not now"
                )
            )
            form(
                submitting: Some(&is_declining),
                attrs: attributes! {
                    class="passkey-setup__decline"
                    method="post"
                    action=(href!(decline_passkey_prompt))
                },
                <button
                    class="passkey-setup__decline-button"
                    type="submit"
                    :disabled=$(is_declining.get())
                >
                    "Don't ask again"
                </button>
            )
            flash_toasts(toast: &toast, flashes: FLASHES)
        </section>
    })
}

#[route(POST "/passkey-setup/snooze")]
pub async fn snooze_passkey_prompt(cx: &Cx) -> Result<SeeOther> {
    let Some(user) = current_user(cx) else {
        return Ok(see_other("/login"));
    };
    query_snooze_passkey_prompt(user.id, SNOOZE_DAYS, db(cx)).await?;
    revoke_passkey_prompt(&session(cx)?).await?;

    Ok(see_other(ACCOUNT_URL))
}

#[route(POST "/passkey-setup/decline")]
pub async fn decline_passkey_prompt(cx: &Cx) -> Result<SeeOther> {
    let Some(user) = current_user(cx) else {
        return Ok(see_other("/login"));
    };
    query_decline_passkey_prompt(user.id, db(cx)).await?;
    revoke_passkey_prompt(&session(cx)?).await?;

    Ok(see_other(ACCOUNT_URL))
}
