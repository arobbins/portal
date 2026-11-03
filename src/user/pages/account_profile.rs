use topcoat::{
    Result,
    view::{View, attributes, component, view},
};

use crate::components::button::{ButtonSize, button};
use crate::components::form::{InputType, form, form_field};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::user::model::User;

#[component]
pub async fn account_profile(user: &User) -> Result<impl View> {
    Ok(view! {
        <div class="account__sub-page-content">
            <h2>"Profile"</h2>
            <section class="account__content-card" id="password">
                <header class="account__content-card-header l-flex l-flex-center-col l-flex-space-between">
                    "Account info"
                    button(
                        size: ButtonSize::Sm,
                        attrs: attributes! { command="show-modal" commandfor="edit-profile" },
                        icon(name: IconName::Edit)
                        "Edit details"
                    )
                </header>
                <div class="account__content-card-details">
                    <p>"Name: " (&user.name)</p>
                    <p>"Email: " (&user.email)</p>
                </div>
            </section>
            modal(
                id: "edit-profile",
                heading: "Edit profile details",
                form(
                    form_field(
                        label: "Name",
                        id: "edit-profile-name",
                        name: "name",
                        value: Some(user.name.as_str()),
                    )
                    form_field(
                        label: "Email",
                        id: "edit-profile-email",
                        name: "email",
                        input_type: InputType::Email,
                        value: Some(user.email.as_str()),
                    )
                )
            )
        </div>
    })
}
