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

use crate::account::tabs::AccountTab;
use crate::action::{ActionProblem, FieldError, browser_caller, require_browser_caller};
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{
    Autocomplete, FORM_ERROR_ID, form, form_actions, form_error, form_field,
};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::toaster::{NO_TOAST, toaster};
use crate::components::tone::Tone;
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};
use crate::profile::actions::{UpdateProfile, update_profile};
use crate::user::model::User;
use crate::user::name::{NAME_FIELD, NameProblem};
use crate::web::session;

const MODAL_ID: &str = "edit-profile";
const FLASHES: &[Flash] = &[
    Flash::ProfileSaved,
    Flash::EmailConfirmed,
    Flash::PasswordReset,
];
const ERROR_TOAST: &str = "error";
const UNREACHABLE: &str = "We couldn't save your profile. Please try again.";

struct ProfileUpdateFailed {
    name: String,
    problem: NameProblem,
}

#[component]
pub async fn profile_tab(cx: &Cx, user: &User, display_name: &Signal<String>) -> Result<impl View> {
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
    let save_failed: std::result::Result<String, FieldError> =
        Err(FieldError::general(UNREACHABLE));
    let is_submitting = signal(cx, || false);

    Ok(view! {
        <div class="p-account__sub-page-content">
            page_heading(text: "Profile", variant: PageHeadingVariant::Compact)
            card(
                id: Some("account-info"),
                card_header(
                    title: "Account info",
                    button(
                        variant: ButtonVariant::Secondary,
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
                        "Edit profile"
                    )
                )
                card_body(
                    <p>
                        "Name: "
                        $(display_name.get())
                    </p>
                    <p>
                        "Email: "
                        (&user.email)
                    </p>
                )
            )
            modal(
                id: MODAL_ID,
                heading: "Edit profile",
                form(
                    attrs: attributes! {
                        method="post"
                        action=(href!(update_profile_submit))
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
                                error_message.set(result.unwrap_err().message);
                            }
                        })
                    },
                    form_field(
                        label: "Name",
                        id: "edit-profile-name",
                        autocomplete: Some(Autocomplete::Name),
                        name: NAME_FIELD,
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
                tone: Tone::Error,
                $(error_message.get())
            )
        </div>
    })
}

#[procedure]
async fn save_profile_name(cx: &Cx, name: String) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };

    match update_profile(cx, &caller, &UpdateProfile { name }).await? {
        Ok(profile) => Ok(Ok(profile.name)),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/profile/update")]
pub async fn update_profile_submit(cx: &Cx, Form(input): Form<UpdateProfile>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let profile_url = AccountTab::Profile.href(cx);

    match update_profile(cx, &caller, &input).await? {
        Ok(_) => {
            set_flash(&session(cx)?, Flash::ProfileSaved).await?;
            Ok(see_other(profile_url))
        }
        Err(error) => Err(rewrite(profile_url, Body::empty())
            .method(Method::GET)
            .with(ProfileUpdateFailed {
                name: input.name,
                problem: error.for_browser()?,
            })
            .into()),
    }
}
