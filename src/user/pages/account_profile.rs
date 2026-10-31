use leptos::prelude::*;

use crate::components::{Button, ButtonSize, Form, FormField, Icon, IconName, InputType, Modal};
use crate::user::model::AccountUser;

#[component]
pub fn AccountProfile(user: AccountUser) -> impl IntoView {
    view! {
        <div class="account__sub-page-content">
            <h2>"Profile"</h2>
            <section class="account__content-card" id="password">
                <header class="account__content-card-header l-flex l-flex-center-col l-flex-space-between">
                    "Account info"
                    <Button size=ButtonSize::Sm command="show-modal" commandfor="edit-profile">
                        <Icon name=IconName::Edit/>
                        "Edit details"
                    </Button>
                </header>
                <div class="account__content-card-details">
                    <p>"Name: " {user.name.clone()}</p>
                    <p>"Email: " {user.email.clone()}</p>
                </div>
            </section>
            <Modal id="edit-profile" heading="Edit profile details">
                <Form>
                    <FormField label="Name" id="edit-profile-name" name="name" value=user.name/>
                    <FormField
                        label="Email"
                        id="edit-profile-email"
                        name="email"
                        input_type=InputType::Email
                        value=user.email
                    />
                </Form>
            </Modal>
        </div>
    }
}
