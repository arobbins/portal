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

use crate::account::pages::account::account_page;
use crate::account::tabs::AccountSection;
use crate::action::require_browser_caller;
use crate::app::page_title;
use crate::auth::session::{grant_passkey_prompt, passkey_prompt_granted, require_user};
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::form::{FormVariant, form, form_error};
use crate::components::icon::{IconName, icon};
use crate::flash::{Flash, flash_toasts, take_flash, toast_name};
use crate::layout::page_heading::page_heading;
use crate::passkeys::actions::{PromptAnswer, answer_passkey_prompt};
use crate::passkeys::pages::passkeys_card::ADD_ERROR_ID;
use crate::passkeys::queries::query_get_passkey_prompt_due;
use crate::user::model::User;
use crate::web::{db, session};

const FLASHES: &[Flash] = &[Flash::EmailConfirmed];

pub async fn after_password_sign_in(cx: &Cx, session: &Session, user: &User) -> Result<String> {
    if query_get_passkey_prompt_due(user.id, db(cx)).await? {
        grant_passkey_prompt(session, user).await?;
        Ok(href!(passkey_setup_page).resolve(cx))
    } else {
        Ok(href!(account_page).resolve(cx))
    }
}

#[page("/passkey-setup", with(Metadata::new().title(page_title("Add passkey"))))]
pub async fn passkey_setup_page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;
    let prompt_ready = passkey_prompt_granted(&session(cx)?, &user).await?
        && query_get_passkey_prompt_due(user.id, db(cx)).await?;
    if !prompt_ready {
        return Err(redirect(href!(account_page).resolve(cx)).into());
    }

    let flash = take_flash(cx, FLASHES).await?;
    let toast = signal(cx, || toast_name(flash).to_string());
    let is_skipping = signal(cx, || false);
    let is_declining = signal(cx, || false);
    let passkeys_href = AccountSection::Passkeys.href(cx);

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

async fn answer_prompt(cx: &Cx, answer: PromptAnswer) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    if let Err(error) = answer_passkey_prompt(cx, &caller, answer).await? {
        match error.for_browser()? {}
    }

    Ok(see_other(href!(account_page).resolve(cx)))
}
