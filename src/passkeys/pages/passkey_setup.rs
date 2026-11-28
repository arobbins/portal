use crate::routes::ACCOUNT_PATH;
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

use crate::account::tabs::AccountSection;
use crate::action::{ForPage, require_browser_caller};
use crate::app::page_title;
use crate::auth::handoff::destination_after_sign_in;
use crate::auth::session::require_user;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::form::{FormVariant, form, form_error};
use crate::components::icon::{IconName, icon};
use crate::flash::{Flash, flash_toast, flash_toasts, take_flash};
use crate::layout::page_heading::page_heading;
use crate::passkeys::actions::passkey_prompt_due;
use crate::passkeys::actions::{PromptAnswer, answer_passkey_prompt};
use crate::passkeys::pages::passkeys_card::ADD_ERROR_ID;
use crate::passkeys::prompt_grant::{grant_passkey_prompt, passkey_prompt_granted};
use crate::user::model::User;
use crate::web::{app, env, session};

const FLASHES: &[Flash] = &[Flash::EmailConfirmed];

pub async fn after_password_sign_in(cx: &Cx, session: &Session, user: &User) -> Result<String> {
    if let Some(destination) = destination_after_sign_in(cx, session, user).await? {
        return Ok(destination);
    }
    let app = app(cx);
    if passkey_prompt_due(&env(cx), user.id).await? {
        grant_passkey_prompt(session, user, app.clock.now()).await?;
        Ok(href!(passkey_setup_page).resolve(cx))
    } else {
        Ok(ACCOUNT_PATH.to_string())
    }
}

#[page("/passkey-setup", with(Metadata::new().title(page_title("Add passkey"))))]
pub async fn passkey_setup_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;
    if !prompt_showing(cx, &user).await? {
        return Err(redirect(ACCOUNT_PATH).into());
    }

    let flash = take_flash(cx, FLASHES).await?;
    let toast = flash_toast(cx, flash);
    let is_skipping = signal(cx, || false);
    let is_declining = signal(cx, || false);
    let passkeys_href = AccountSection::Passkeys.path();

    Ok(view! {
        <section class="l-contain l-skinny p-passkey-setup">
            <div class="p-passkey-setup__badge">icon(name: IconName::Passkey)</div>
            page_heading(text: "Sign in faster next time")
            <p class="p-auth__intro">
                "Save a passkey and sign in with your fingerprint, face or screen lock instead of typing your password. Passkeys can't be phished or reused on other sites."
            </p>
            form(
                variant: FormVariant::Page,
                attrs: attributes! {
                    class="p-passkey-setup__add"
                    data-passkey-register=""
                    data-passkey-only=""
                    data-passkey-done=(passkeys_href.as_str())
                },
                form_error(id: Some(ADD_ERROR_ID), hidden: true, "")
                button(button_type: ButtonType::Submit, "Add passkey")
            )
            <noscript>
                <p class="p-auth__intro">"Adding a passkey needs JavaScript."</p>
            </noscript>
            form(
                submitting: Some(&is_skipping),
                attrs: attributes! {
                    class="p-passkey-setup__skip"
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
                    class="p-passkey-setup__decline"
                    method="post"
                    action=(href!(decline_passkey_prompt))
                },
                <button
                    class="p-passkey-setup__decline-button"
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
    answer_prompt(cx, PromptAnswer::NotNow).await
}

#[route(POST "/passkey-setup/decline")]
pub async fn decline_passkey_prompt(cx: &Cx) -> Result<SeeOther> {
    answer_prompt(cx, PromptAnswer::Never).await
}

async fn prompt_showing(cx: &Cx, user: &User) -> Result<bool> {
    let now = app(cx).clock.now();
    Ok(passkey_prompt_granted(&session(cx)?, user, now).await?
        && passkey_prompt_due(&env(cx), user.id).await?)
}

async fn answer_prompt(cx: &Cx, answer: PromptAnswer) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    if !prompt_showing(cx, &caller.user).await? {
        return Ok(see_other(ACCOUNT_PATH));
    }
    answer_passkey_prompt(&env(cx), &session(cx)?, &caller, answer)
        .await?
        .for_page()?;

    Ok(see_other(ACCOUNT_PATH))
}
