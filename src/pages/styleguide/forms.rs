use leptos::prelude::*;

use super::StyleguideSection;
use crate::components::{Button, ButtonType, Form, FormCol, FormField, FormRow, InputType};

#[component]
pub fn ExampleFormFields(#[prop(into)] id_prefix: String) -> impl IntoView {
    let first_name_id = format!("{id_prefix}-first-name");
    let email_id = format!("{id_prefix}-email");
    let password_id = format!("{id_prefix}-password");
    let confirm_password_id = format!("{id_prefix}-confirm-password");

    view! {
        <FormField label="First name" id=first_name_id name="first_name" placeholder="Andrew"/>
        <FormField
            label="Email"
            id=email_id
            name="email"
            input_type=InputType::Email
            placeholder="hello@andrewrobbins.dev"
        />
        <FormRow>
            <FormCol half=true>
                <FormField
                    label="New password"
                    id=password_id
                    name="password"
                    input_type=InputType::Password
                    placeholder="Enter a password"
                    required=true
                />
            </FormCol>
            <FormCol half=true>
                <FormField
                    label="Confirm password"
                    id=confirm_password_id
                    name="confirm_password"
                    input_type=InputType::Password
                    placeholder="Confirm your password"
                    required=true
                />
            </FormCol>
        </FormRow>
    }
}

#[component]
pub fn FormsSection() -> impl IntoView {
    view! {
        <StyleguideSection title="Forms">
            <Form>
                <ExampleFormFields id_prefix="styleguide-form"/>
                <Button button_type=ButtonType::Submit>"Submit"</Button>
            </Form>
        </StyleguideSection>
    }
}
