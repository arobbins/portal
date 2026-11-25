use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use super::delete_routes::{delete_account_js, delete_account_submit};
use crate::account::tabs::AccountSection;
use crate::auth::actions::CONFIRMATION_FIELD;
use crate::auth::pages::login::login_page;
use crate::auth::pages::reauth_field::reauth_field;
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::billing::pages::plan_page::subscription_to_cancel;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::form::{
    DialogForm, ModalSubmit, dialog_input, form_field, form_group, modal_form,
};
use crate::components::notice::notice;
use crate::components::property_list::property;
use crate::components::tone::Tone;
use crate::data_export::pages::export_card::export_first_prompt;
use crate::flash::Flash;

const MODAL_ID: &str = "delete-account-dialog";
const UNREACHABLE: &str = "We couldn't delete your account. Please try again.";

pub struct DeleteDrafts {
    blank: Signal<String>,
    confirmation: Signal<String>,
    password: Signal<String>,
}

impl DeleteDrafts {
    pub fn new(cx: &Cx) -> Self {
        Self {
            blank: signal(cx, String::new),
            confirmation: signal(cx, String::new),
            password: signal(cx, String::new),
        }
    }
}

#[component]
pub async fn delete_account_row(
    cx: &Cx,
    form: &DialogForm,
    drafts: &DeleteDrafts,
) -> Result<impl View> {
    let DeleteDrafts {
        blank,
        confirmation,
        password,
    } = drafts;
    let DialogForm {
        error_field,
        error_message,
        ..
    } = form;
    let section = AccountSection::DeleteAccount;

    Ok(view! {
        property(
            name: section.label(),
            id: Some(section.anchor()),
            description: Some(
                "Permanently delete your account, passkeys and API tokens. This happens right away and can't be undone.",
            ),
            button(
                variant: ButtonVariant::Danger,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    command="show-modal"
                    commandfor=(MODAL_ID)
                    @click=$(|_e: Event| {
                        blank.set("".to_owned());
                        confirmation.set("".to_owned());
                        password.set("".to_owned());
                        error_field.set("".to_owned());
                        error_message.set("".to_owned());
                    })
                },
                "Delete account"
            )
        )
    })
}

#[component]
pub async fn delete_account_dialog(
    cx: &Cx,
    form: &DialogForm,
    drafts: &DeleteDrafts,
    toast: &Signal<String>,
) -> Result<impl View> {
    let DeleteDrafts {
        blank,
        confirmation,
        password,
    } = drafts;
    let login = href!(login_page).resolve(cx);
    let cancelled = subscription_to_cancel(cx)
        .await?
        .map(|plan| format!("Your {plan} subscription is cancelled now, with no refund."));
    let submit = ModalSubmit {
        procedure: delete_account_js,
        action: href!(delete_account_submit).resolve(cx),
        label: "Delete account",
        success_toast: Flash::AccountDeleted.name(),
        unreachable: UNREACHABLE,
    };

    Ok(view! {
        modal_form(
            id: MODAL_ID,
            heading: "Delete account",
            form: form,
            toast: toast,
            submit: submit,
            args: $((confirmation.get(), password.get())),
            variant: ButtonVariant::Danger,
            navigate_to: Some(&login),
            form_group(
                notice(
                    tone: Tone::Error,
                    "Your account, passkeys and API tokens are deleted right away. This can't be undone."
                    if let Some(cancelled) = &cancelled {
                        " "
                        (cancelled.as_str())
                    }
                )
            )
            export_first_prompt(from_modal: MODAL_ID)
            form_field(
                label: "Type DELETE to confirm",
                id: "delete-account-confirmation",
                name: CONFIRMATION_FIELD,
                required: true,
                attrs: attributes! {
                    autofocus=""
                    autocomplete="off"
                    autocapitalize="characters"
                    spellcheck="false"
                    (dialog_input(cx, form, CONFIRMATION_FIELD, blank, confirmation))
                }
            )
            reauth_field(
                id: "delete-account-password",
                suggest_passkey: false,
                attrs: dialog_input(cx, form, CURRENT_PASSWORD_FIELD, blank, password)
            )
        )
    })
}
