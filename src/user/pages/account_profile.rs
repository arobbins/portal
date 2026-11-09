use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_request_context},
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        href, route,
    },
    runtime::{Event, Signal, procedure, signal},
    view::{View, attributes, component, view},
};

use crate::components::button::{ButtonSize, ButtonType, button};
use crate::components::form::{Autocomplete, form, form_field};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::notice::{NoticeKind, notice};
use crate::components::toaster::{NO_TOAST, toaster};
use crate::flash::{Flash, set_flash, take_flash};
use crate::user::auth::current_user;
use crate::user::model::User;
use crate::user::queries::query_update_user_name;
use crate::web::{db, session};

const PROFILE_URL: &str = "/account/profile";
const MODAL_ID: &str = "edit-profile";
const MAX_NAME_LENGTH: usize = 100;
const SAVED_TOAST: &str = "saved";
const VERIFIED_TOAST: &str = "verified";
const ERROR_TOAST: &str = "error";
const PASSWORD_RESET_TOAST: &str = "password-reset";
const UNREACHABLE: &str = "We couldn't save your profile. Please try again.";

#[derive(Deserialize)]
pub struct ProfileInput {
    name: String,
}

struct ProfileUpdateFailed {
    name: String,
    message: &'static str,
}

#[component]
pub async fn account_profile(
    cx: &Cx,
    user: &User,
    display_name: &Signal<String>,
) -> Result<impl View> {
    let flash = take_flash(
        cx,
        &[
            Flash::ProfileSaved,
            Flash::EmailConfirmed,
            Flash::PasswordReset,
        ],
    )
    .await?;
    let failed = try_request_context::<ProfileUpdateFailed>(cx);
    let name_value = failed.map_or_else(|| user.name.clone(), |failed| failed.name.clone());

    let draft_name = signal(cx, || name_value.clone());
    let error_message = signal(cx, || {
        failed.map_or_else(String::new, |failed| failed.message.to_string())
    });
    let initial_toast = if failed.is_some() {
        ERROR_TOAST
    } else if flash == Some(Flash::ProfileSaved) {
        SAVED_TOAST
    } else if flash == Some(Flash::EmailConfirmed) {
        VERIFIED_TOAST
    } else if flash == Some(Flash::PasswordReset) {
        PASSWORD_RESET_TOAST
    } else {
        NO_TOAST
    };
    let toast = signal(cx, || initial_toast.to_string());
    let is_submitting = signal(cx, || false);
    let error_class = "account__profile-error";
    let hidden_error_class = "account__profile-error account__profile-error--is-hidden";

    Ok(view! {
        <div class="account__sub-page-content">
            <h2 class="account__sub-page-heading">"Profile"</h2>
            <section class="account__content-card" id="password">
                <header
                    class="account__content-card-header l-flex l-flex-center-col l-flex-space-between"
                >
                    "Account info"
                    button(
                        size: ButtonSize::Sm,
                        attrs: attributes! {
                            command="show-modal"
                            commandfor=(MODAL_ID)
                            @click=$(|_e: Event| {
                                draft_name.set(display_name.get());
                                error_message.set("".to_owned());
                            })
                        },
                        icon(name: IconName::Edit)
                        "Edit details"
                    )
                </header>
                <div class="account__content-card-details">
                    <p>
                        "Name: "
                        $(display_name.get())
                    </p>
                    <p>
                        "Email: "
                        (&user.email)
                    </p>
                </div>
            </section>
            modal(
                id: MODAL_ID,
                heading: "Edit profile details",
                form(
                    attrs: attributes! {
                        method="post"
                        action=(href!(update_profile))
                        @submit=$(async |e: Event| {
                            e.prevent_default();
                            is_submitting.set(true);
                            error_message.set("".to_owned());
                            toast.set(NO_TOAST.to_owned());
                            let unreachable = UNREACHABLE.to_owned();
                            let pending = save_profile_name(draft_name.get());
                            let idle = false;
                            raw!(
                                "${pending} = Promise.resolve(${pending}).catch(() => {
                                ${is_submitting}.set(${idle});
                                ${error_message}.set(${unreachable});
                                return new Promise(() => {});
                            });",
                                drop((unreachable, idle)),
                            );
                            let result = pending.await;
                            is_submitting.set(false);
                            if result.is_ok() {
                                display_name.set(result.unwrap());
                                toast.set(SAVED_TOAST.to_owned());
                                raw!("document.getElementById('edit-profile').close()", ());
                            } else {
                                error_message.set(result.unwrap_err());
                            }
                        })
                    },
                    form_field(
                        label: "Name",
                        id: "edit-profile-name",
                        autocomplete: Some(Autocomplete::Name),
                        name: "name",
                        required: true,
                        input_attrs: attributes! {
                            :value=$(draft_name.get())
                            @input=$(|e: Event| draft_name.set(e.target.value))
                        }
                    )
                    <div
                        :class=$(if error_message.get().is_empty() {
                            hidden_error_class
                        } else {
                            error_class
                        })
                    >
                        notice(kind: NoticeKind::Error, $(error_message.get()))
                    </div>
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        disabled: $(is_submitting.get()),
                        "Save profile"
                    )
                )
            )
            toaster(
                toast: &toast,
                name: SAVED_TOAST,
                kind: NoticeKind::Success,
                "Profile saved."
            )
            toaster(
                toast: &toast,
                name: VERIFIED_TOAST,
                kind: NoticeKind::Success,
                "Your email is confirmed. Welcome!"
            )
            toaster(
                toast: &toast,
                name: PASSWORD_RESET_TOAST,
                kind: NoticeKind::Success,
                "Your password has been reset."
            )
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
async fn save_profile_name(cx: &Cx, name: String) -> Result<Result<String, String>> {
    let Some(user) = current_user(cx) else {
        return Ok(Err(
            "Your session has expired. Please log in again.".to_string()
        ));
    };

    Ok(update_name(cx, &user, &name).await?.map_err(str::to_string))
}

#[route(POST "/account/profile/update")]
pub async fn update_profile(cx: &Cx, Form(input): Form<ProfileInput>) -> Result<SeeOther> {
    let Some(user) = current_user(cx) else {
        return Ok(see_other("/login"));
    };

    match update_name(cx, &user, &input.name).await? {
        Ok(_) => {
            set_flash(&session(cx)?, Flash::ProfileSaved).await?;
            Ok(see_other(PROFILE_URL))
        }
        Err(message) => Err(rewrite(PROFILE_URL, Body::empty())
            .method(Method::GET)
            .with(ProfileUpdateFailed {
                name: input.name,
                message,
            })
            .into()),
    }
}

async fn update_name(cx: &Cx, user: &User, name: &str) -> Result<Result<String, &'static str>> {
    let name = name.trim();

    if name.is_empty() {
        return Ok(Err("Name is required."));
    }

    if name.chars().count() > MAX_NAME_LENGTH {
        return Ok(Err("Name must be 100 characters or fewer."));
    }

    query_update_user_name(user.id, name, db(cx)).await?;

    Ok(Ok(name.to_string()))
}
