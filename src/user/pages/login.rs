use leptos::prelude::*;

use super::redirect_signed_in_users;
use crate::components::{Button, ButtonType, FormField, InputType, Notice, NoticeKind};
use crate::layout::PageHeading;
use crate::user::api::Login;
use crate::web::error_message;

#[component]
pub fn LoginPage() -> impl IntoView {
    redirect_signed_in_users();

    view! {
        <section class="l-contain l-skinny">
            <PageHeading text="Login"/>
            <LoginForm/>
        </section>
    }
}

#[island]
fn LoginForm() -> impl IntoView {
    let login = ServerAction::<Login>::new();

    let error = move || {
        login.value().get().and_then(Result::err).map(|error| {
            view! { <Notice kind=NoticeKind::Error>{error_message(&error)}</Notice> }
        })
    };

    view! {
        <ActionForm action=login attr:class="a-form form form--skinny">
            <FormField
                label="Email"
                id="email"
                name="email"
                input_type=InputType::Email
                placeholder="Enter email"
            />
            <FormField
                label="Password"
                id="password"
                name="password"
                input_type=InputType::Password
                placeholder="Enter password"
            />
            <Button button_type=ButtonType::Submit busy=login.pending() disabled=login.pending()>
                "Login"
            </Button>
            {error}
        </ActionForm>
    }
}
