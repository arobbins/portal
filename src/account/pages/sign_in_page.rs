use topcoat::{
    Result,
    context::{Cx, try_context},
    router::href,
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use super::password_routes::{ChangePasswordFailed, change_password_submit, save_new_password};
use super::placeholder_page::coming_soon_card;
use crate::account::tabs::AccountSection;
use crate::action::ActionProblem;
use crate::auth::pages::new_password_form::new_password_fields;
use crate::auth::password::{CONFIRMATION_FIELD, CURRENT_PASSWORD_FIELD, PASSWORD_FIELD};
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{Autocomplete, InputType, form_field};
use crate::components::form::{
    DialogForm, ModalSubmit, dialog_error_toast, dialog_input, modal_form, page_toast,
};
use crate::components::icon::{IconName, icon};
use crate::flash::{Flash, flash_toasts, take_flash};
use crate::passkeys::pages::passkey_routes::{remove_problem, rename_problem};
use crate::passkeys::pages::passkeys_card::passkeys_card;
use crate::two_factor::pages::two_factor_card::two_factor_card;
use crate::two_factor::pages::two_factor_routes::two_factor_failure;

const MODAL_ID: &str = "change-password";
const UNREACHABLE: &str = "We couldn't change your password. Please try again.";
const FLASHES: &[Flash] = &[
    Flash::PasswordChanged,
    Flash::PasskeyAdded,
    Flash::PasskeyRenamed,
    Flash::PasskeyRemoved,
    Flash::TwoFactorDisabled,
];

struct PasswordDrafts {
    blank: Signal<String>,
    current: Signal<String>,
    password: Signal<String>,
    confirmation: Signal<String>,
}

impl PasswordDrafts {
    fn new(cx: &Cx) -> Self {
        Self {
            blank: signal(cx, String::new),
            current: signal(cx, String::new),
            password: signal(cx, String::new),
            confirmation: signal(cx, String::new),
        }
    }
}

#[component]
pub async fn sign_in_page(cx: &Cx) -> Result<impl View> {
    let flash = take_flash(cx, FLASHES).await?;
    let password_failure =
        try_context::<ChangePasswordFailed>(cx).map(|failed| failed.problem.spec());
    let failure = password_failure
        .or_else(|| rename_problem(cx).map(ActionProblem::spec))
        .or_else(|| remove_problem(cx).map(ActionProblem::spec))
        .or_else(|| two_factor_failure(cx).map(|failed| failed.problem.spec()));
    let form = DialogForm::new(cx, password_failure);
    let toast = page_toast(cx, failure.is_some(), flash);
    let drafts = PasswordDrafts::new(cx);

    Ok(view! {
        <div class="p-account__sub-page-content">
            password_card(form: &form, drafts: &drafts)
            passkeys_card()
            two_factor_card()
            coming_soon_card(
                section: AccountSection::SingleSignOn,
                action: "Connect",
                text: "Sign in with your company's identity provider. Coming soon."
            )
            change_password_dialog(form: &form, drafts: &drafts, toast: &toast)
            flash_toasts(toast: &toast, flashes: FLASHES)
            dialog_error_toast(failure: failure, toast: &toast)
        </div>
    })
}

#[component]
async fn password_card(cx: &Cx, form: &DialogForm, drafts: &PasswordDrafts) -> Result<impl View> {
    let PasswordDrafts {
        blank,
        current,
        password,
        confirmation,
    } = drafts;
    let DialogForm {
        error_field,
        error_message,
        ..
    } = form;

    Ok(view! {
        card(
            id: Some(AccountSection::Password.anchor()),
            card_header(
                title: AccountSection::Password.label(),
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        command="show-modal"
                        commandfor=(MODAL_ID)
                        @click=$(|_e: Event| {
                            blank.set("".to_owned());
                            current.set("".to_owned());
                            password.set("".to_owned());
                            confirmation.set("".to_owned());
                            error_field.set("".to_owned());
                            error_message.set("".to_owned());
                        })
                    },
                    icon(name: IconName::Edit)
                    "Change password"
                )
            )
            card_body(
                <p>
                    "Use a long password you don't use anywhere else. Changing it signs you out on your other devices."
                </p>
            )
        )
    })
}

#[component]
async fn change_password_dialog(
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
