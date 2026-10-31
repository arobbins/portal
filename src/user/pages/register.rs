use leptos::prelude::*;

use super::redirect_signed_in_users;
use crate::components::{Button, ButtonType, FormField, InputType, Notice, NoticeKind};
use crate::layout::PageHeading;
use crate::user::api::Register;
use crate::web::error_message;

#[component]
pub fn RegisterPage() -> impl IntoView {
    redirect_signed_in_users();

    view! {
        <section class="l-contain l-skinny">
            <PageHeading text="Register"/>
            <RegisterForm/>
        </section>
    }
}

#[island]
fn RegisterForm() -> impl IntoView {
    let register = ServerAction::<Register>::new();

    let result = move || {
        register.value().get().map(|result| match result {
            Ok(message) => view! { <Notice kind=NoticeKind::Success>{message}</Notice> }.into_any(),
            Err(error) => {
                view! { <Notice kind=NoticeKind::Error>{error_message(&error)}</Notice> }.into_any()
            }
        })
    };

    view! {
        <ActionForm action=register attr:class="a-form form form--skinny">
            <FormField label="Name" id="name" name="name" placeholder="Name"/>
            <FormField label="Email" id="email" name="email" input_type=InputType::Email placeholder="Email"/>
            <FormField
                label="New password"
                id="new-password"
                name="new_password"
                input_type=InputType::Password
                placeholder="Enter a password"
            />
            <FormField
                label="Confirm new password"
                id="confirm-password"
                name="confirm_password"
                input_type=InputType::Password
                placeholder="Confirm password"
            />
            <Button button_type=ButtonType::Submit busy=register.pending() disabled=register.pending()>
                "Register account"
            </Button>
            {result}
        </ActionForm>
    }
}
