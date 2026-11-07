use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_request_context},
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, rewrite, see_other},
        href, query_params, route,
    },
    runtime::{Event, Signal, procedure, signal},
    view::{View, attributes, component, view},
};

use crate::components::button::{ButtonSize, ButtonType, button};
use crate::components::form::{form, form_field};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::notice::NoticeKind;
use crate::components::toaster::toaster;
use crate::user::auth::current_user;
use crate::user::model::User;
use crate::user::queries::query_update_user_name;
use crate::web::{CurrentUser, db, session};

const PROFILE_URL: &str = "/account/profile";
const MODAL_ID: &str = "edit-profile";
const MAX_NAME_LENGTH: usize = 100;

#[derive(Deserialize)]
pub struct ProfileInput {
    name: String,
}

struct ProfileUpdateFailed {
    name: String,
    message: &'static str,
}

#[query_params]
struct ProfileQuery {
    saved: Option<bool>,
}

#[component]
pub async fn account_profile(cx: &Cx, user: &User, display_name: &Signal<String>) -> Result<impl View> {
    let saved = query_params::<ProfileQuery>(cx)
        .ok()
        .and_then(|query| query.saved)
        .unwrap_or(false);
    let failed = try_request_context::<ProfileUpdateFailed>(cx);
    let name_value = failed.map_or_else(|| user.name.clone(), |failed| failed.name.clone());

    let draft_name = signal(cx, || name_value.clone());
    let error_message = signal(cx, || failed.map_or_else(String::new, |failed| failed.message.to_string()));
    let is_saved_open = signal(cx, || saved);
    let is_error_open = signal(cx, || failed.is_some());
    let is_submitting = signal(cx, || false);

    Ok(view! {
        <div class="account__sub-page-content">
            <h2 class="account__sub-page-heading">"Profile"</h2>
            <section class="account__content-card" id="password">
                <header class="account__content-card-header l-flex l-flex-center-col l-flex-space-between">
                    "Account info"
                    button(
                        size: ButtonSize::Sm,
                        attrs: attributes! { command="show-modal" commandfor=(MODAL_ID) },
                        icon(name: IconName::Edit)
                        "Edit details"
                    )
                </header>
                <div class="account__content-card-details">
                    <p>"Name: " $(display_name.get())</p>
                    <p>"Email: " (&user.email)</p>
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
                            is_saved_open.set(false);
                            is_error_open.set(false);
                            let result = save_profile_name(draft_name.get()).await;
                            is_submitting.set(false);
                            if result.is_ok() {
                                display_name.set(result.unwrap());
                                is_saved_open.set(true);
                                raw!("document.getElementById('edit-profile').close()", ());
                            } else {
                                error_message.set(result.unwrap_err());
                                is_error_open.set(true);
                            }
                        })
                    },
                    form_field(
                        label: "Name",
                        id: "edit-profile-name",
                        name: "name",
                        value: Some(name_value.as_str()),
                        required: true,
                        input_attrs: attributes! {
                            @input=$(|e: Event| draft_name.set(e.target.value))
                        },
                    )
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        disabled: $(is_submitting.get()),
                        "Save profile"
                    )
                )
            )
            toaster(open: &is_saved_open, kind: NoticeKind::Success, "Profile saved.")
            toaster(open: &is_error_open, kind: NoticeKind::Error, $(error_message.get()))
        </div>
    })
}

#[procedure]
async fn save_profile_name(cx: &Cx, name: String) -> Result<Result<String, String>> {
    let Some(user) = current_user(cx).await? else {
        return Ok(Err("Your session has expired. Please log in again.".to_string()));
    };

    Ok(update_name(cx, &user, &name).await?.map_err(str::to_string))
}

#[route(POST "/account/profile/update")]
pub async fn update_profile(cx: &Cx, Form(input): Form<ProfileInput>) -> Result<SeeOther> {
    let Some(user) = current_user(cx).await? else {
        return Ok(see_other("/login"));
    };

    match update_name(cx, &user, &input.name).await? {
        Ok(_) => Ok(see_other(format!("{PROFILE_URL}?saved=true"))),
        Err(message) => Err(rewrite(PROFILE_URL, Body::empty())
            .method(Method::GET)
            .with(ProfileUpdateFailed { name: input.name, message })
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
    session(cx)?
        .insert("current_user", CurrentUser { name: name.to_string() })
        .await?;

    Ok(Ok(name.to_string()))
}
