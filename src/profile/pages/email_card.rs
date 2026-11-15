use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use super::email_routes::{change_email_submit, send_email_change_link};
use crate::auth::actions::NEW_EMAIL_FIELD;
use crate::auth::pages::reauth_field::reauth_field;
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{Autocomplete, InputType, form_field};
use crate::components::form::{DialogForm, ModalSubmit, dialog_input, modal_form};
use crate::components::icon::{IconName, icon};
use crate::flash::Flash;

const UNREACHABLE: &str = "We couldn't send the confirmation link. Please try again.";
const MODAL_ID: &str = "change-email";

pub struct EmailDrafts {
    blank: Signal<String>,
    new_email: Signal<String>,
    password: Signal<String>,
}

impl EmailDrafts {
    pub fn new(cx: &Cx) -> Self {
        Self {
            blank: signal(cx, String::new),
            new_email: signal(cx, String::new),
            password: signal(cx, String::new),
        }
    }
}

#[component]
pub async fn email_card(
    cx: &Cx,
    email: &str,
    form: &DialogForm,
    drafts: &EmailDrafts,
) -> Result<impl View> {
    let EmailDrafts {
        blank,
        new_email,
        password,
        ..
    } = drafts;
    let DialogForm {
        error_field,
        error_message,
        ..
    } = form;

    Ok(view! {
        card(
            id: Some("email"),
            card_header(
                title: "Email",
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        command="show-modal"
                        commandfor=(MODAL_ID)
                        @click=$(|_e: Event| {
                            blank.set("".to_owned());
                            new_email.set("".to_owned());
                            password.set("".to_owned());
                            error_field.set("".to_owned());
                            error_message.set("".to_owned());
                        })
                    },
                    icon(name: IconName::Edit)
                    "Change email"
                )
            )
            card_body(
                <p>
                    "You sign in with "
                    <strong>(email)</strong>
                    "."
                </p>
                <p>
                    "To change it, we'll send a link to the new address. Nothing changes until you open it."
                </p>
            )
        )
    })
}

#[component]
pub async fn change_email_dialog(
    cx: &Cx,
    form: &DialogForm,
    email: &str,
    drafts: &EmailDrafts,
    toast: &Signal<String>,
) -> Result<impl View> {
    let EmailDrafts {
        new_email,
        password,
        ..
    } = drafts;
    let submit = ModalSubmit {
        procedure: send_email_change_link,
        action: href!(change_email_submit).resolve(cx),
        label: "Send confirmation link",
        success_toast: Flash::EmailChangeSent.name(),
        unreachable: UNREACHABLE,
    };

    Ok(view! {
        modal_form(
            id: MODAL_ID,
            heading: "Change email",
            form: form,
            toast: toast,
            submit: submit,
            args: $((new_email.get(), password.get())),
            email_fields(form: form, email: email, drafts: drafts)
        )
    })
}

#[component]
async fn email_fields(
    cx: &Cx,
    form: &DialogForm,
    email: &str,
    drafts: &EmailDrafts,
) -> Result<impl View> {
    let EmailDrafts {
        blank,
        new_email,
        password,
        ..
    } = drafts;

    Ok(view! {
        form_field(
            label: "Current email",
            id: "change-email-current",
            name: "current_email",
            input_type: InputType::Email,
            value: Some(email),
            attrs: attributes! { readonly="" }
        )
        form_field(
            label: "New email",
            id: "change-email-new",
            autocomplete: Some(Autocomplete::Email),
            name: NEW_EMAIL_FIELD,
            input_type: InputType::Email,
            required: true,
            attrs: attributes! {
                autofocus=""
                (dialog_input(cx, form, NEW_EMAIL_FIELD, blank, new_email))
            }
        )
        reauth_field(
            id: "change-email-password",
            attrs: dialog_input(cx, form, CURRENT_PASSWORD_FIELD, blank, password)
        )
    })
}
