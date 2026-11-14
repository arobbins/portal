use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        href, route,
    },
    runtime::{Event, procedure, signal},
    view::{View, attributes, component, view},
};

use crate::account::tabs::{AccountSection, AccountTab};
use crate::action::{ActionProblem, FieldError, browser_caller, require_browser_caller};
use crate::auth::actions::{ChangePassword, ChangePasswordProblem, change_password};
use crate::auth::pages::new_password_form::new_password_fields;
use crate::auth::password::{CONFIRMATION_FIELD, CURRENT_PASSWORD_FIELD, PASSWORD_FIELD};
use crate::auth::session::sign_in;
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{
    Autocomplete, FORM_ERROR_ID, InputType, form, form_actions, form_error, form_field,
};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::toaster::{NO_TOAST, toaster};
use crate::components::tone::Tone;
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};
use crate::passkeys::pages::passkeys_card::{passkeys_card, rename_problem};
use crate::web::session;

const MODAL_ID: &str = "change-password";
const FLASHES: &[Flash] = &[
    Flash::PasswordChanged,
    Flash::PasskeyAdded,
    Flash::PasskeyRenamed,
    Flash::PasskeyRemoved,
];
const ERROR_TOAST: &str = "error";
const UNREACHABLE: &str = "We couldn't change your password. Please try again.";

struct ChangePasswordFailed {
    problem: ChangePasswordProblem,
}

#[component]
pub async fn security_tab(cx: &Cx) -> Result<impl View> {
    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<ChangePasswordFailed>(cx).map(|failed| failed.problem);

    let blank = signal(cx, String::new);
    let current_draft = signal(cx, String::new);
    let password_draft = signal(cx, String::new);
    let confirmation_draft = signal(cx, String::new);
    let error_field = signal(cx, || {
        failed
            .and_then(ActionProblem::field)
            .unwrap_or_default()
            .to_string()
    });
    let rename_failed = rename_problem(cx);
    let error_message = signal(cx, || {
        failed
            .map(ActionProblem::message)
            .or(rename_failed.map(ActionProblem::message))
            .unwrap_or("")
            .to_string()
    });
    let initial_toast = if failed.is_some() || rename_failed.is_some() {
        ERROR_TOAST
    } else {
        toast_name(flash)
    };
    let toast = signal(cx, || initial_toast.to_string());
    let is_submitting = signal(cx, || false);

    let changed_toast = Flash::PasswordChanged.name().to_string();
    let modal_id = MODAL_ID;
    let current_field = CURRENT_PASSWORD_FIELD.to_string();
    let password_field = PASSWORD_FIELD.to_string();
    let confirmation_field = CONFIRMATION_FIELD.to_string();
    let change_failed: std::result::Result<bool, FieldError> =
        Err(FieldError::general(UNREACHABLE));

    Ok(view! {
        <div class="p-account__sub-page-content">
            page_heading(text: "Security", variant: PageHeadingVariant::Compact)
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
                                current_draft.set("".to_owned());
                                password_draft.set("".to_owned());
                                confirmation_draft.set("".to_owned());
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
            passkeys_card()
            card(
                id: Some(AccountSection::TwoFactor.anchor()),
                card_header(
                    title: AccountSection::TwoFactor.label(),
                    button(
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        disabled: true,
                        "Set up"
                    )
                )
                card_body(
                    <p>
                        "Confirm it's you with an authenticator app when you sign in. Coming soon."
                    </p>
                )
            )
            card(
                id: Some(AccountSection::SingleSignOn.anchor()),
                card_header(
                    title: AccountSection::SingleSignOn.label(),
                    button(
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        disabled: true,
                        "Connect"
                    )
                )
                card_body(
                    <p>"Sign in with your company's identity provider. Coming soon."</p>
                )
            )
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
                                current_draft.get(),
                                password_draft.get(),
                                confirmation_draft.get(),
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
                    form_field(
                        label: "Current password",
                        id: "current-password",
                        autocomplete: Some(Autocomplete::CurrentPassword),
                        name: CURRENT_PASSWORD_FIELD,
                        input_type: InputType::Password,
                        required: true,
                        attrs: attributes! {
                            :value=$(blank.get())
                            :aria-invalid=$(if error_field.get() == current_field {
                                "true"
                            } else {
                                "false"
                            })
                            aria-describedby=(FORM_ERROR_ID)
                            @input=$(|e: Event| current_draft.set(e.target.value))
                        }
                    )
                    new_password_fields(
                        password_attrs: attributes! {
                            :value=$(blank.get())
                            :aria-invalid=$(if error_field.get() == password_field {
                                "true"
                            } else {
                                "false"
                            })
                            aria-describedby=(FORM_ERROR_ID)
                            @input=$(|e: Event| password_draft.set(e.target.value))
                        },
                        confirmation_attrs: attributes! {
                            :value=$(blank.get())
                            :aria-invalid=$(if error_field.get() == confirmation_field {
                                "true"
                            } else {
                                "false"
                            })
                            aria-describedby=(FORM_ERROR_ID)
                            @input=$(|e: Event| confirmation_draft.set(e.target.value))
                        }
                    )
                    form_error(
                        hidden: $(error_message.get().is_empty()),
                        $(error_message.get())
                    )
                    form_actions(
                        button(
                            button_type: ButtonType::Submit,
                            busy: $(is_submitting.get()),
                            "Change password"
                        )
                    )
                )
            )
            flash_toasts(toast: &toast, flashes: FLASHES)
            toaster(
                toast: &toast,
                name: ERROR_TOAST,
                tone: Tone::Error,
                $(error_message.get())
            )
        </div>
    })
}

#[procedure]
async fn save_new_password(
    cx: &Cx,
    current_password: String,
    password: String,
    confirm_password: String,
) -> Result<Result<bool, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let input = ChangePassword {
        current_password,
        password,
        confirm_password,
    };

    match change_password(cx, &caller, &input).await? {
        Ok(user) => {
            sign_in(&session(cx)?, &user).await?;
            Ok(Ok(true))
        }
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/password/change")]
pub async fn change_password_submit(
    cx: &Cx,
    Form(input): Form<ChangePassword>,
) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    match change_password(cx, &caller, &input).await? {
        Ok(user) => {
            let session = session(cx)?;
            sign_in(&session, &user).await?;
            set_flash(&session, Flash::PasswordChanged).await?;
            Ok(see_other(AccountTab::Security.href(cx)))
        }
        Err(error) => Err(rewrite(AccountTab::Security.href(cx), Body::empty())
            .method(Method::GET)
            .with(ChangePasswordFailed {
                problem: error.for_browser()?,
            })
            .into()),
    }
}
