use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_context},
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
use crate::components::form::{
    Autocomplete, FORM_ERROR_ID, form, form_actions, form_error, form_field,
};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::notice::NoticeKind;
use crate::components::toaster::{NO_TOAST, toaster};
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::user::auth::current_user;
use crate::user::model::User;
use crate::user::name::{NameProblem, check_name};
use crate::user::queries::query_update_user_name;
use crate::web::{db, session};

const PROFILE_URL: &str = "/account/profile";
const MODAL_ID: &str = "edit-profile";
const FLASHES: &[Flash] = &[
    Flash::ProfileSaved,
    Flash::EmailConfirmed,
    Flash::PasswordReset,
];
const ERROR_TOAST: &str = "error";
const SESSION_EXPIRED: &str = "Your session has expired. Please log in again.";
const UNREACHABLE: &str = "We couldn't save your profile. Please try again.";

#[derive(Deserialize)]
pub struct ProfileInput {
    name: String,
}

struct ProfileUpdateFailed {
    name: String,
    problem: NameProblem,
}

#[component]
pub async fn account_profile(
    cx: &Cx,
    user: &User,
    display_name: &Signal<String>,
) -> Result<impl View> {
    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<ProfileUpdateFailed>(cx);
    let name_value = failed.map_or_else(|| user.name.clone(), |failed| failed.name.clone());

    let field_value = signal(cx, || name_value.clone());
    let draft_name = signal(cx, || name_value.clone());
    let error_message = signal(cx, || {
        failed.map_or_else(String::new, |failed| failed.problem.message().to_string())
    });
    let initial_toast = if failed.is_some() {
        ERROR_TOAST
    } else {
        toast_name(flash)
    };
    let toast = signal(cx, || initial_toast.to_string());
    let saved_toast = Flash::ProfileSaved.name().to_string();
    let modal_id = MODAL_ID;
    let save_failed: std::result::Result<String, String> = Err(UNREACHABLE.to_string());
    let is_submitting = signal(cx, || false);

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
                                field_value.set(display_name.get());
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
                            let _pending = save_profile_name(draft_name.get());
                            let result = raw!(
                                "await Promise.resolve(${_pending}).catch(() => ${save_failed})",
                                save_failed.clone(),
                            );
                            is_submitting.set(false);
                            if result.is_ok() {
                                display_name.set(result.unwrap());
                                toast.set(saved_toast.clone());
                                raw!(
                                    "document.getElementById(String(${modal_id})).close()",
                                    {
                                        let _ = modal_id;
                                    },
                                );
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
                        attrs: attributes! {
                            :value=$(field_value.get())
                            :aria-invalid=$(if error_message.get().is_empty() {
                                "false"
                            } else {
                                "true"
                            })
                            aria-describedby=(FORM_ERROR_ID)
                            @input=$(|e: Event| draft_name.set(e.target.value))
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
                            "Save profile"
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
async fn save_profile_name(cx: &Cx, name: String) -> Result<Result<String, String>> {
    let Some(user) = current_user(cx) else {
        return Ok(Err(SESSION_EXPIRED.to_string()));
    };

    Ok(update_name(cx, &user, &name)
        .await?
        .map_err(|problem| problem.message().to_string()))
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
        Err(problem) => Err(rewrite(PROFILE_URL, Body::empty())
            .method(Method::GET)
            .with(ProfileUpdateFailed {
                name: input.name,
                problem,
            })
            .into()),
    }
}

async fn update_name(cx: &Cx, user: &User, name: &str) -> Result<Result<String, NameProblem>> {
    let name = match check_name(name) {
        Ok(name) => name,
        Err(problem) => return Ok(Err(problem)),
    };

    query_update_user_name(user.id, name, db(cx)).await?;

    Ok(Ok(name.to_string()))
}
