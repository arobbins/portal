use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::Signal,
    view::{View, attributes, component, view},
};

use super::token_fields::{TokenDrafts, choice_fields};
use super::token_routes::{create_api_token_js, create_api_token_submit};
use crate::account::tabs::AccountTab;
use crate::auth::pages::reauth_field::reauth_field;
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::components::button::{ButtonType, button};
use crate::components::copy_field::copy_field;
use crate::components::form::{
    DialogForm, ModalSubmit, ModalSuccess, SubmitSettings, dialog_input,
};
use crate::components::form::{dialog_submit_form, form, form_actions, form_field};
use crate::components::modal::modal;
use crate::components::notice::notice;
use crate::components::toaster::NO_TOAST;
use crate::components::tone::Tone;
use crate::user::name::{MAX_NAME_LENGTH, NAME_FIELD};

pub(super) const CREATE_MODAL: &str = "create-api-token";
const UNREACHABLE: &str = "We couldn't create the token. Please try again.";
const STEP_CLASS: &str = "p-account__api-token-step";
const HIDDEN_STEP_CLASS: &str = "p-account__api-token-step p-account__api-token-step--is-hidden";
const REVEAL_CLASS: &str = "p-account__api-token-step p-account__api-token-reveal";
const HIDDEN_REVEAL_CLASS: &str =
    "p-account__api-token-step p-account__api-token-reveal p-account__api-token-step--is-hidden";

#[component]
pub(super) async fn create_token_dialog(
    cx: &Cx,
    form: &DialogForm,
    drafts: &TokenDrafts,
    toast: &Signal<String>,
) -> Result<impl View> {
    let created = &drafts.created;

    Ok(view! {
        modal(
            id: CREATE_MODAL,
            heading: "Create API token",
            <div
                :class=$(if created.get().is_empty() {
                    STEP_CLASS
                } else {
                    HIDDEN_STEP_CLASS
                })
            >
                create_token_form(form: form, drafts: drafts, toast: toast)
            </div>
            <div
                :class=$(if created.get().is_empty() {
                    HIDDEN_REVEAL_CLASS
                } else {
                    REVEAL_CLASS
                })
            >
                new_token_reveal(created: created)
            </div>
        )
    })
}

#[component]
async fn create_token_form(
    cx: &Cx,
    form: &DialogForm,
    drafts: &TokenDrafts,
    toast: &Signal<String>,
) -> Result<impl View> {
    let TokenDrafts {
        blank,
        name,
        access,
        lifetime,
        audience,
        password,
        created,
    } = drafts;
    let submit = ModalSubmit {
        procedure: create_api_token_js,
        action: href!(create_api_token_submit).resolve(cx),
        label: "Create API token",
        success_toast: NO_TOAST,
        unreachable: UNREACHABLE,
    };
    let settings = SubmitSettings {
        modal_id: CREATE_MODAL,
        on_success: ModalSuccess::KeepOpen,
        saved: Some(created),
        clear: Some((blank, password)),
    };

    Ok(view! {
        dialog_submit_form(
            form: form,
            toast: toast,
            submit: submit,
            args: $((
                name.get(),
                access.get(),
                lifetime.get(),
                audience.get(),
                password.get(),
            )),
            settings: settings,
            text_fields(form: form, drafts: drafts)
            choice_fields(drafts: drafts)
            password_field(form: form, drafts: drafts)
        )
    })
}

#[component]
async fn text_fields(cx: &Cx, form: &DialogForm, drafts: &TokenDrafts) -> Result<impl View> {
    let TokenDrafts { blank, name, .. } = drafts;

    Ok(view! {
        form_field(
            label: "Name",
            id: "api-token-name",
            name: NAME_FIELD,
            required: true,
            placeholder: Some("e.g. Deploy script"),
            max_length: Some(MAX_NAME_LENGTH),
            attrs: dialog_input(cx, form, NAME_FIELD, blank, name)
        )
    })
}

#[component]
async fn password_field(cx: &Cx, form: &DialogForm, drafts: &TokenDrafts) -> Result<impl View> {
    let TokenDrafts {
        blank, password, ..
    } = drafts;

    Ok(view! {
        reauth_field(
            id: "api-token-password",
            attrs: dialog_input(cx, form, CURRENT_PASSWORD_FIELD, blank, password)
        )
    })
}

#[component]
async fn new_token_reveal(cx: &Cx, created: &Signal<String>) -> Result<impl View> {
    let developer_url = AccountTab::Developer.path();

    Ok(view! {
        notice(
            tone: Tone::Warning,
            "Copy this token now. You won't be able to see it again."
        )
        copy_field(label: "API token", id: "api-token-value", source: Some(created))
        form(
            attrs: attributes! { method="get" action=(developer_url.as_str()) },
            form_actions(button(button_type: ButtonType::Submit, "Done"))
        )
    })
}
