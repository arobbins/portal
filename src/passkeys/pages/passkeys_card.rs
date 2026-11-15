use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::signal,
    view::{View, attributes, component, view},
};

use super::passkey_routes::{remove_passkey_submit, rename_passkey_submit};
use super::passkey_view::PasskeyRow;
use crate::account::tabs::AccountSection;
use crate::action::{ID_FIELD, require_browser_caller};
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{
    Autocomplete, InputType, form, form_actions, form_error, form_field,
};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::notice::notice;
use crate::components::property_list::{PropertyTag, property, property_list};
use crate::components::tone::Tone;
use crate::passkeys::actions::list_passkeys;
use crate::user::name::{MAX_NAME_LENGTH, NAME_FIELD};
use crate::web::env;

const ADD_MODAL: &str = "add-passkey";
const RENAME_MODAL: &str = "rename-passkey";
const REMOVE_MODAL: &str = "remove-passkey";
pub const ADD_ERROR_ID: &str = "add-passkey-error";

#[component]
pub async fn passkeys_card(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let passkeys = match list_passkeys(&env(cx), &caller).await? {
        Ok(passkeys) => passkeys,
        Err(error) => match error.for_browser()? {},
    };
    let rows: Vec<PasskeyRow> = passkeys.iter().map(PasskeyRow::from_passkey).collect();
    let only_one = passkeys.len() == 1;
    let needs_backup = only_one && !passkeys[0].backup_eligible;

    Ok(view! {
        card(
            id: Some(AccountSection::Passkeys.anchor()),
            card_header(
                title: "Passkeys",
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! { command="show-modal" commandfor=(ADD_MODAL) },
                    icon(name: IconName::Plus)
                    "Add passkey"
                )
            )
            card_body(
                if rows.is_empty() {
                    <p>
                        "Sign in with your fingerprint, face or screen lock instead of typing your password. Passkeys can't be phished or reused on other sites."
                    </p>
                } else {
                    property_list(
                        #[key(row.id.as_str())]
                        for row in &rows {
                            passkey_item(row: row)
                        }
                    )
                }
                if needs_backup {
                    notice(
                        tone: Tone::Warning,
                        "This passkey only exists on one device. Add a second passkey so you can still sign in if you lose it."
                    )
                }
            )
        )
        #[key(row.id.as_str())]
        for row in &rows {
            passkey_dialogs(row: row, only_one: only_one)
        }
        modal(
            id: ADD_MODAL,
            heading: "Add passkey",
            form(
                attrs: attributes! { data-passkey-register="" },
                <p>"Confirm your password, then follow your device's prompt."</p>
                form_field(
                    label: "Current password",
                    id: "add-passkey-password",
                    autocomplete: Some(Autocomplete::CurrentPassword),
                    name: CURRENT_PASSWORD_FIELD,
                    input_type: InputType::Password,
                    required: true,
                    attrs: attributes! { aria-describedby=(ADD_ERROR_ID) }
                )
                form_error(id: Some(ADD_ERROR_ID), hidden: true, "")
                <noscript><p>"Adding a passkey needs JavaScript."</p></noscript>
                form_actions(button(button_type: ButtonType::Submit, "Continue"))
            )
        )
    })
}

#[component]
async fn passkey_item(cx: &Cx, row: &PasskeyRow) -> Result<impl View> {
    let rename_modal = format!("{RENAME_MODAL}-{}", row.id);
    let remove_modal = format!("{REMOVE_MODAL}-{}", row.id);
    let rename_label = format!("Rename {}", row.name);
    let remove_label = format!("Remove {}", row.name);
    let tags = if row.synced {
        vec![PropertyTag::new("Synced", Tone::Neutral)]
    } else {
        vec![PropertyTag::new("This device only", Tone::Warning)]
    };

    Ok(view! {
        property(
            name: row.name.as_str(),
            tags: &tags,
            detail: Some(row.summary.as_str()),
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    command="show-modal"
                    commandfor=(rename_modal.as_str())
                    aria-label=(rename_label)
                },
                "Rename"
            )
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    command="show-modal"
                    commandfor=(remove_modal.as_str())
                    aria-label=(remove_label)
                },
                "Remove"
            )
        )
    })
}

#[component]
async fn passkey_dialogs(cx: &Cx, row: &PasskeyRow, only_one: bool) -> Result<impl View> {
    let rename_modal = format!("{RENAME_MODAL}-{}", row.id);
    let remove_modal = format!("{REMOVE_MODAL}-{}", row.id);
    let name_field = format!("passkey-name-{}", row.id);

    let is_renaming = signal(cx, || false);
    let is_removing = signal(cx, || false);

    Ok(view! {
        modal(
            id: &rename_modal,
            heading: "Rename passkey",
            form(
                submitting: Some(&is_renaming),
                attrs: attributes! { method="post" action=(href!(rename_passkey_submit)) },
                <input type="hidden" name=(ID_FIELD) value=(row.id.as_str())>
                form_field(
                    label: "Name",
                    id: &name_field,
                    name: NAME_FIELD,
                    value: Some(row.name.as_str()),
                    required: true,
                    max_length: Some(MAX_NAME_LENGTH)
                )
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_renaming.get()),
                        "Rename passkey"
                    )
                )
            )
        )
        modal(
            id: &remove_modal,
            heading: "Remove passkey",
            form(
                submitting: Some(&is_removing),
                attrs: attributes! { method="post" action=(href!(remove_passkey_submit)) },
                <input type="hidden" name=(ID_FIELD) value=(row.id.as_str())>
                <p>
                    "Remove "
                    <strong>(row.name.as_str())</strong>
                    "? You won't be able to sign in with it anymore."
                </p>
                if only_one {
                    <p>
                        "This is your only passkey. You'll still be able to sign in with your password."
                    </p>
                }
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_removing.get()),
                        "Remove passkey"
                    )
                )
            )
        )
    })
}
