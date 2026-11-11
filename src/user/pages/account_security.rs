use serde::Deserialize;
use topcoat::{
    Error, Result,
    context::{Cx, try_context},
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        href,
        request::client_ip,
        route,
    },
    runtime::{Event, procedure, signal},
    view::{View, attributes, component, view},
};

use super::account_passkeys::{account_passkeys, rename_problem};
use super::new_password_form::new_password_fields;
use crate::components::button::{ButtonSize, ButtonType, button};
use crate::components::form::{
    Autocomplete, FORM_ERROR_ID, InputType, form, form_actions, form_error, form_field,
};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::notice::NoticeKind;
use crate::components::toaster::{NO_TOAST, toaster};
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::user::auth::{current_user, hash_password, password_matches, sign_in};
use crate::user::model::{AccountStatus, User};
use crate::user::name::NameProblem;
use crate::user::password::{PasswordProblem, check_new_password};
use crate::user::queries::query_update_password;
use crate::web::{db, rate_limits, session};

const SECURITY_URL: &str = "/account/security";
const MODAL_ID: &str = "change-password";
const FLASHES: &[Flash] = &[
    Flash::PasswordChanged,
    Flash::PasskeyAdded,
    Flash::PasskeyRenamed,
    Flash::PasskeyRemoved,
];
const ERROR_TOAST: &str = "error";
const CURRENT_FIELD: &str = "current_password";
const PASSWORD_FIELD: &str = "password";
const CONFIRMATION_FIELD: &str = "confirm_password";
const SESSION_EXPIRED: &str = "Your session has expired. Please log in again.";
const UNREACHABLE: &str = "We couldn't change your password. Please try again.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChangePasswordProblem {
    WrongCurrentPassword,
    NewPassword(PasswordProblem),
    TooManyAttempts,
}

impl ChangePasswordProblem {
    fn field(self) -> &'static str {
        match self {
            Self::WrongCurrentPassword => CURRENT_FIELD,
            Self::NewPassword(problem) if problem.invalid_confirmation() => CONFIRMATION_FIELD,
            Self::NewPassword(_) => PASSWORD_FIELD,
            Self::TooManyAttempts => "",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::WrongCurrentPassword => "Your current password is incorrect.",
            Self::NewPassword(problem) => problem.message(),
            Self::TooManyAttempts => "Too many attempts. Please wait a few minutes and try again.",
        }
    }
}

struct ChangePasswordFailed {
    problem: ChangePasswordProblem,
}

#[component]
pub async fn account_security(cx: &Cx) -> Result<impl View> {
    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<ChangePasswordFailed>(cx).map(|failed| failed.problem);

    let blank = signal(cx, String::new);
    let current_draft = signal(cx, String::new);
    let password_draft = signal(cx, String::new);
    let confirmation_draft = signal(cx, String::new);
    let error_field = signal(cx, || {
        failed.map_or("", ChangePasswordProblem::field).to_string()
    });
    let rename_failed = rename_problem(cx);
    let error_message = signal(cx, || {
        failed
            .map(ChangePasswordProblem::message)
            .or(rename_failed.map(NameProblem::message))
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
    let current_field = CURRENT_FIELD.to_string();
    let password_field = PASSWORD_FIELD.to_string();
    let confirmation_field = CONFIRMATION_FIELD.to_string();
    let change_failed: std::result::Result<bool, (String, String)> =
        Err((String::new(), UNREACHABLE.to_string()));

    Ok(view! {
        <div class="account__sub-page-content">
            <h2 class="account__sub-page-heading">"Security"</h2>
            <section class="account__content-card" id="password">
                <header
                    class="account__content-card-header l-flex l-flex-center-col l-flex-space-between"
                >
                    "Password"
                    button(
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
                </header>
                <div class="account__content-card-details">
                    <p>
                        "Use a long password you don't use anywhere else. Changing it signs you out on your other devices."
                    </p>
                </div>
            </section>
            account_passkeys()
            <section class="account__content-card" id="2fa">
                <header
                    class="account__content-card-header l-flex l-flex-center-col l-flex-space-between"
                >
                    "Two-factor authentication"
                    button(size: ButtonSize::Sm, disabled: true, "Set up")
                </header>
                <div class="account__content-card-details">
                    <p>
                        "Confirm it's you with an authenticator app when you sign in. Coming soon."
                    </p>
                </div>
            </section>
            <section class="account__content-card" id="sso">
                <header
                    class="account__content-card-header l-flex l-flex-center-col l-flex-space-between"
                >
                    "Single sign-on"
                    button(size: ButtonSize::Sm, disabled: true, "Connect")
                </header>
                <div class="account__content-card-details">
                    <p>"Sign in with your company's identity provider. Coming soon."</p>
                </div>
            </section>
            modal(
                id: MODAL_ID,
                heading: "Change password",
                form(
                    attrs: attributes! {
                        method="post"
                        action=(href!(change_password))
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
                                error_field.set(failure.0);
                                error_message.set(failure.1);
                            }
                        })
                    },
                    form_field(
                        label: "Current password",
                        id: "current-password",
                        autocomplete: Some(Autocomplete::CurrentPassword),
                        name: CURRENT_FIELD,
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
                kind: NoticeKind::Error,
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
) -> Result<Result<bool, (String, String)>> {
    let Some(user) = current_user(cx) else {
        return Ok(Err((String::new(), SESSION_EXPIRED.to_string())));
    };
    let input = ChangePasswordInput {
        current_password,
        password,
        confirm_password,
    };

    Ok(change_signed_in_password(cx, &user, &input)
        .await?
        .map(|()| true)
        .map_err(|problem| (problem.field().to_string(), problem.message().to_string())))
}

#[derive(Deserialize)]
pub struct ChangePasswordInput {
    current_password: String,
    password: String,
    confirm_password: String,
}

#[route(POST "/account/password")]
pub async fn change_password(cx: &Cx, Form(input): Form<ChangePasswordInput>) -> Result<SeeOther> {
    let Some(user) = current_user(cx) else {
        return Ok(see_other("/login"));
    };

    match change_signed_in_password(cx, &user, &input).await? {
        Ok(()) => {
            set_flash(&session(cx)?, Flash::PasswordChanged).await?;
            Ok(see_other(SECURITY_URL))
        }
        Err(problem) => Err(rewrite(SECURITY_URL, Body::empty())
            .method(Method::GET)
            .with(ChangePasswordFailed { problem })
            .into()),
    }
}

async fn change_signed_in_password(
    cx: &Cx,
    user: &User,
    input: &ChangePasswordInput,
) -> Result<std::result::Result<(), ChangePasswordProblem>> {
    let limits = rate_limits(cx);
    let ip = client_ip(cx);

    if !limits.allow_password_submission(ip) || limits.login_blocked(&user.email, ip) {
        return Ok(Err(ChangePasswordProblem::TooManyAttempts));
    }

    let current_hash = match user.status() {
        AccountStatus::Active { password_hash } => Some(password_hash),
        AccountStatus::AwaitingPassword => None,
    };
    if !password_matches(current_hash, &input.current_password) {
        limits.record_failed_login(&user.email, ip);
        return Ok(Err(ChangePasswordProblem::WrongCurrentPassword));
    }

    if let Err(problem) = check_new_password(&input.password, &input.confirm_password) {
        return Ok(Err(ChangePasswordProblem::NewPassword(problem)));
    }

    let password_hash = hash_password(&input.password).map_err(|e| Error::msg(e.to_string()))?;
    let updated = query_update_password(user.id, &password_hash, db(cx))
        .await?
        .ok_or_else(|| Error::msg("signed-in user is not active"))?;

    limits.clear_failed_logins(&user.email, ip);
    sign_in(&session(cx)?, &updated).await?;

    Ok(Ok(()))
}
