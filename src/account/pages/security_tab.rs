use topcoat::{
    Result,
    context::{Cx, try_context},
    router::href,
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use super::password_routes::{ChangePasswordFailed, change_password_submit, save_new_password};
use crate::account::tabs::AccountSection;
use crate::action::{ActionProblem, FieldError};
use crate::auth::pages::new_password_form::new_password_fields;
use crate::auth::password::{CONFIRMATION_FIELD, CURRENT_PASSWORD_FIELD, PASSWORD_FIELD};
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{Autocomplete, InputType, form, form_actions, form_field};
use crate::components::form::{DialogForm, dialog_error_toast, dialog_form_error, dialog_input};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::toaster::NO_TOAST;
use crate::flash::{Flash, flash_toasts, take_flash, toast_name};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};
use crate::passkeys::pages::passkeys_card::{passkeys_card, rename_problem};

const MODAL_ID: &str = "change-password";
const FLASHES: &[Flash] = &[
    Flash::PasswordChanged,
    Flash::PasskeyAdded,
    Flash::PasskeyRenamed,
    Flash::PasskeyRemoved,
];
const ERROR_TOAST: &str = "error";
const UNREACHABLE: &str = "We couldn't change your password. Please try again.";

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
pub async fn security_tab(cx: &Cx) -> Result<impl View> {
    let flash = take_flash(cx, FLASHES).await?;
    let failure = try_context::<ChangePasswordFailed>(cx)
        .map(|failed| failed.problem.spec())
        .or_else(|| rename_problem(cx).map(ActionProblem::spec));
    let form = DialogForm::new(
        cx,
        failure.and_then(|spec| spec.field).unwrap_or_default(),
        failure.map_or("", |spec| spec.message),
    );
    let initial_toast = if failure.is_some() {
        ERROR_TOAST
    } else {
        toast_name(flash)
    };
    let toast = signal(cx, || initial_toast.to_string());
    let drafts = PasswordDrafts::new(cx);

    Ok(view! {
        <div class="p-account__sub-page-content">
            page_heading(text: "Security", variant: PageHeadingVariant::Compact)
            password_card(form: &form, drafts: &drafts)
            passkeys_card()
            coming_soon_card(
                section: AccountSection::TwoFactor,
                action: "Set up",
                text: "Confirm it's you with an authenticator app when you sign in. Coming soon."
            )
            coming_soon_card(
                section: AccountSection::SingleSignOn,
                action: "Connect",
                text: "Sign in with your company's identity provider. Coming soon."
            )
            change_password_dialog(form: &form, drafts: &drafts, toast: &toast)
            flash_toasts(toast: &toast, flashes: FLASHES)
            dialog_error_toast(form: &form, toast: &toast, name: ERROR_TOAST)
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
async fn coming_soon_card(
    cx: &Cx,
    section: AccountSection,
    action: &str,
    text: &str,
) -> Result<impl View> {
    Ok(view! {
        card(
            id: Some(section.anchor()),
            card_header(
                title: section.label(),
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    disabled: true,
                    (action)
                )
            )
            card_body(<p>(text)</p>)
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
    let DialogForm {
        error_field,
        error_message,
        is_submitting,
    } = form;
    let PasswordDrafts {
        current,
        password,
        confirmation,
        ..
    } = drafts;
    let changed_toast = Flash::PasswordChanged.name().to_string();
    let modal_id = MODAL_ID;
    let change_failed: std::result::Result<bool, FieldError> =
        Err(FieldError::general(UNREACHABLE));

    Ok(view! {
        modal(
            id: MODAL_ID,
            heading: "Change password",
            form(
                attrs: attributes! {
                    method="post"
                    action=(href!(change_password_submit))
                    @submit=$(async |e: Event| {
                        e.prevent_default();
                        is_submitting.set(true);
                        error_field.set("".to_owned());
                        error_message.set("".to_owned());
                        toast.set(NO_TOAST.to_owned());
                        let _pending = save_new_password(
                            current.get(),
                            password.get(),
                            confirmation.get(),
                        );
                        let result = raw!(
                            "await Promise.resolve(${_pending}).catch(() => ${change_failed})",
                            change_failed.clone(),
                        );
                        is_submitting.set(false);
                        if result.is_ok() {
                            toast.set(changed_toast.clone());
                            raw!(
                                "document.getElementById(String(${modal_id})).close()",
                                {
                                    let _ = modal_id;
                                },
                            );
                        } else {
                            let failure = result.unwrap_err();
                            error_field.set(failure.field);
                            error_message.set(failure.message);
                        }
                    })
                },
                password_fields(form: form, drafts: drafts)
                dialog_form_error(form: form)
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Change password"
                    )
                )
            )
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
