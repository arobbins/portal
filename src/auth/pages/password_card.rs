use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Signal, signal},
    view::{View, component, view},
};

use super::password_routes::{change_password_submit, save_new_password};
use crate::account::tabs::AccountSection;
use crate::action::{ForPage, require_browser_caller};
use crate::auth::actions::{PasswordAge, password_age};
use crate::auth::pages::new_password_form::new_password_fields;
use crate::auth::password::{CONFIRMATION_FIELD, CURRENT_PASSWORD_FIELD, PASSWORD_FIELD};
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{Autocomplete, InputType, form_field};
use crate::components::form::{DialogForm, ModalSubmit, dialog_input, dialog_opener, modal_form};
use crate::components::icon::{IconName, icon};
use crate::flash::Flash;
use crate::web::env;

const MODAL_ID: &str = "change-password";
const UNREACHABLE: &str = "We couldn't change your password. Please try again.";

pub struct PasswordDrafts {
    blank: Signal<String>,
    current: Signal<String>,
    password: Signal<String>,
    confirmation: Signal<String>,
}

impl PasswordDrafts {
    pub fn new(cx: &Cx) -> Self {
        Self {
            blank: signal(cx, String::new),
            current: signal(cx, String::new),
            password: signal(cx, String::new),
            confirmation: signal(cx, String::new),
        }
    }
}

#[component]
pub async fn password_card(
    cx: &Cx,
    form: &DialogForm,
    drafts: &PasswordDrafts,
) -> Result<impl View> {
    let note = password_note(cx).await?;
    let PasswordDrafts {
        blank,
        current,
        password,
        confirmation,
    } = drafts;

    Ok(view! {
        card(
            id: Some(AccountSection::Password.anchor()),
            card_header(
                title: AccountSection::Password.label(),
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: dialog_opener(
                        cx,
                        form,
                        MODAL_ID,
                        &[blank, current, password, confirmation],
                    ),
                    icon(name: IconName::Edit)
                    "Change password"
                )
            )
            card_body(
                <p class="p-account__card-note">(note)</p>
                <p>
                    "Use a long password you don't use anywhere else. Changing it signs you out on your other devices."
                </p>
            )
        )
    })
}

#[component]
pub async fn change_password_dialog(
    cx: &Cx,
    form: &DialogForm,
    drafts: &PasswordDrafts,
    toast: &Signal<String>,
) -> Result<impl View> {
    let PasswordDrafts {
        current,
        password,
        confirmation,
        ..
    } = drafts;
    let submit = ModalSubmit {
        procedure: save_new_password,
        action: href!(change_password_submit).resolve(cx),
        label: "Change password",
        success_toast: Flash::PasswordChanged.name(),
        unreachable: UNREACHABLE,
    };

    Ok(view! {
        modal_form(
            id: MODAL_ID,
            heading: "Change password",
            form: form,
            toast: toast,
            submit: submit,
            args: $((current.get(), password.get(), confirmation.get())),
            password_fields(form: form, drafts: drafts)
        )
    })
}

#[component]
async fn password_fields(cx: &Cx, form: &DialogForm, drafts: &PasswordDrafts) -> Result<impl View> {
    let PasswordDrafts {
        blank,
        current,
        password,
        confirmation,
    } = drafts;

    Ok(view! {
        form_field(
            label: "Current password",
            id: "current-password",
            autocomplete: Some(Autocomplete::CurrentPassword),
            name: CURRENT_PASSWORD_FIELD,
            input_type: InputType::Password,
            required: true,
            attrs: dialog_input(cx, form, CURRENT_PASSWORD_FIELD, blank, current)
        )
        new_password_fields(
            password_attrs: dialog_input(cx, form, PASSWORD_FIELD, blank, password),
            confirmation_attrs: dialog_input(
                cx,
                form,
                CONFIRMATION_FIELD,
                blank,
                confirmation,
            )
        )
    })
}

async fn password_note(cx: &Cx) -> Result<String> {
    let caller = require_browser_caller(cx)?;
    let age = password_age(&env(cx), &caller).await?.for_page()?;
    let zone = caller.user.zone();
    Ok(match age {
        PasswordAge::Changed(at) => format!("Last changed {}.", zone.date(at)),
        PasswordAge::Set(at) => format!("Set {}.", zone.date(at)),
        PasswordAge::OverAYear => "Last changed more than a year ago.".to_string(),
    })
}
