use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, route,
    },
    runtime::{Event, Signal, procedure, signal},
    view::{View, attributes, component, view},
};

use crate::account::tabs::AccountTab;
use crate::action::{ActionProblem, FieldError, browser_caller, require_browser_caller};
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{Autocomplete, form, form_actions, form_field};
use crate::components::form::{DialogForm, dialog_error_toast, dialog_form_error, dialog_input};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::toaster::NO_TOAST;
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};
use crate::profile::actions::{UpdateProfile, update_profile};
use crate::user::model::User;
use crate::user::name::{NAME_FIELD, NameProblem};
use crate::web::{env, redisplay, session};

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

struct NameDrafts {
    value: Signal<String>,
    draft: Signal<String>,
}

#[component]
pub async fn profile_tab(cx: &Cx, user: &User, display_name: &Signal<String>) -> Result<impl View> {
    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<ProfileUpdateFailed>(cx);
    let name = failed.map_or_else(|| user.name.clone(), |failed| failed.name.clone());
    let form = DialogForm::new(cx, "", failed.map_or("", |failed| failed.problem.message()));
    let initial_toast = if failed.is_some() {
        ERROR_TOAST
    } else {
        toast_name(flash)
    };
    let toast = signal(cx, || initial_toast.to_string());
    let drafts = NameDrafts {
        value: signal(cx, || name.clone()),
        draft: signal(cx, || name.clone()),
    };

    Ok(view! {
        <div class="p-account__sub-page-content">
            page_heading(text: "Profile", variant: PageHeadingVariant::Compact)
            profile_card(
                user: user,
                display_name: display_name,
                form: &form,
                drafts: &drafts
            )
            edit_profile_dialog(
                form: &form,
                drafts: &drafts,
                display_name: display_name,
                toast: &toast
            )
            flash_toasts(toast: &toast, flashes: FLASHES)
            dialog_error_toast(form: &form, toast: &toast, name: ERROR_TOAST)
        </div>
    })
}

#[component]
async fn profile_card(
    cx: &Cx,
    user: &User,
    display_name: &Signal<String>,
    form: &DialogForm,
    drafts: &NameDrafts,
) -> Result<impl View> {
    let NameDrafts { value, draft } = drafts;
    let error_message = &form.error_message;

    Ok(view! {
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
                            value.set(display_name.get());
                            draft.set(display_name.get());
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
    })
}

#[component]
async fn edit_profile_dialog(
    cx: &Cx,
    form: &DialogForm,
    drafts: &NameDrafts,
    display_name: &Signal<String>,
    toast: &Signal<String>,
) -> Result<impl View> {
    let DialogForm {
        error_message,
        is_submitting,
        ..
    } = form;
    let NameDrafts { value, draft } = drafts;
    let saved_toast = Flash::ProfileSaved.name().to_string();
    let modal_id = MODAL_ID;
    let save_failed: std::result::Result<String, FieldError> =
        Err(FieldError::general(UNREACHABLE));

    Ok(view! {
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
                        let _pending = save_profile_name(draft.get());
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
                    attrs: dialog_input(cx, form, NAME_FIELD, value, draft)
                )
                dialog_form_error(form: form)
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Save profile"
                    )
                )
            )
        )
    })
}

#[procedure]
async fn save_profile_name(cx: &Cx, name: String) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };

    match update_profile(&env(cx), &caller, &UpdateProfile { name }).await? {
        Ok(profile) => Ok(Ok(profile.name)),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/profile/update")]
pub async fn update_profile_submit(cx: &Cx, Form(input): Form<UpdateProfile>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    match update_profile(&env(cx), &caller, &input).await? {
        Ok(_) => {
            set_flash(&session(cx)?, Flash::ProfileSaved).await?;
            Ok(see_other(AccountTab::Profile.path()))
        }
        Err(error) => {
            let problem = error.for_browser()?;
            let failed = ProfileUpdateFailed {
                name: input.name,
                problem,
            };
            Err(redisplay(AccountTab::Profile.path(), failed))
        }
    }
}
