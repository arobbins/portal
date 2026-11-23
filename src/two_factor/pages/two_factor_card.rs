use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, attributes, component, view},
};

use super::two_factor_routes::{
    disable_two_factor_submit, enable_two_factor_submit, regenerate_recovery_codes_submit,
    shown_recovery_codes, two_factor_failure,
};
use crate::account::tabs::AccountSection;
use crate::action::require_browser_caller;
use crate::auth::pages::reauth_field::reauth_field;
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::code_block::{code_block, code_panel};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::copy_field::copy_field;
use crate::components::form::{Autocomplete, form, form_actions, form_field};
use crate::components::modal::{modal, modal_cancel};
use crate::components::tone::Tone;
use crate::two_factor::actions::{CODE_FIELD, get_two_factor};
use crate::two_factor::codes::{new_secret, qr_code_image, setup_url};
use crate::web::env;

const SETUP_MODAL: &str = "set-up-two-factor";
const CODES_MODAL: &str = "new-recovery-codes";
const OFF_MODAL: &str = "turn-off-two-factor";

#[component]
pub async fn two_factor_card(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let summary = match get_two_factor(&env(cx), &caller).await? {
        Ok(summary) => summary,
        Err(error) => match error.for_browser()? {},
    };
    let section = AccountSection::TwoFactor;
    let codes = shown_recovery_codes(cx).map(|shown| shown.codes.join("\n"));
    let status = summary.enabled_at.map(|enabled_at| {
        format!(
            "On since {}. {} of 10 recovery codes left.",
            caller.user.zone().date(enabled_at),
            summary.recovery_codes_left
        )
    });

    Ok(view! {
        card(
            id: Some(section.anchor()),
            card_header(
                title: section.label(),
                status: Some(
                    if status.is_some() {
                        ("On", Tone::Success)
                    } else {
                        ("Off", Tone::Neutral)
                    },
                ),
                two_factor_actions(on: status.is_some())
            )
            card_body(
                if let Some(status) = &status {
                    <p>(status)</p>
                } else {
                    <p>
                        "Ask for a code from an authenticator app when you sign in with your password. Signing in with a passkey doesn't need one."
                    </p>
                }
                if let Some(codes) = &codes {
                    <p>
                        "Save these recovery codes somewhere safe, like a password manager. Each one works once if you lose your phone. You won't see them again."
                    </p>
                    code_panel(
                        title: "Recovery codes",
                        code_block(code: codes.as_str())
                    )
                }
            )
        )
        if status.is_some() {
            change_dialogs()
        } else {
            setup_dialog()
        }
    })
}

#[component]
async fn two_factor_actions(cx: &Cx, on: bool) -> Result<impl View> {
    Ok(view! {
        if on {
            <div class="p-account__card-actions">
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! { command="show-modal" commandfor=(CODES_MODAL) },
                    "New recovery codes"
                )
                button(
                    variant: ButtonVariant::Danger,
                    size: ButtonSize::Sm,
                    attrs: attributes! { command="show-modal" commandfor=(OFF_MODAL) },
                    "Turn off"
                )
            </div>
        } else {
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                attrs: attributes! { command="show-modal" commandfor=(SETUP_MODAL) },
                "Set up"
            )
        }
    })
}

#[component]
async fn setup_dialog(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let secret = two_factor_failure(cx)
        .and_then(|failed| failed.secret.clone())
        .unwrap_or_else(new_secret);
    let qr = setup_url(&secret, &caller.user.email)
        .and_then(|url| qr_code_image(&url))
        .unwrap_or_default();
    let action = href!(enable_two_factor_submit).resolve(cx);

    Ok(view! {
        modal(
            id: SETUP_MODAL,
            heading: "Set up two-factor authentication",
            form(
                attrs: attributes! { method="post" action=(action) },
                <p>
                    "Scan this QR code with an authenticator app, such as 1Password, Google Authenticator or Authy."
                </p>
                <img
                    class="p-account__qr-code"
                    src=(qr)
                    alt="QR code for your authenticator app"
                    width="200"
                    height="200"
                >
                copy_field(
                    label: "Or enter this setup key",
                    id: "two-factor-secret",
                    value: &secret
                )
                <input type="hidden" name="secret" value=(secret.as_str())>
                form_field(
                    label: "Code from the app",
                    id: "two-factor-setup-code",
                    name: CODE_FIELD,
                    autocomplete: Some(Autocomplete::OneTimeCode),
                    required: true,
                    attrs: attributes! { inputmode="numeric" }
                )
                reauth_field(id: "two-factor-setup-password", suggest_passkey: false)
                form_actions(
                    modal_cancel(modal_id: SETUP_MODAL)
                    button(button_type: ButtonType::Submit, "Turn on two-factor")
                )
            )
        )
    })
}

#[component]
async fn change_dialogs(cx: &Cx) -> Result<impl View> {
    let codes_action = href!(regenerate_recovery_codes_submit).resolve(cx);
    let off_action = href!(disable_two_factor_submit).resolve(cx);

    Ok(view! {
        confirm_dialog(
            id: CODES_MODAL,
            heading: "Create new recovery codes",
            confirm_label: "Create new codes",
            action: Some(&codes_action),
            <p>"Your old recovery codes stop working."</p>
            reauth_field(id: "new-recovery-codes-password", suggest_passkey: false)
        )
        confirm_dialog(
            id: OFF_MODAL,
            heading: "Turn off two-factor authentication",
            confirm_label: "Turn off",
            action: Some(&off_action),
            <p>"Signing in with your password will no longer ask for a code."</p>
            reauth_field(id: "turn-off-two-factor-password", suggest_passkey: false)
        )
    })
}
