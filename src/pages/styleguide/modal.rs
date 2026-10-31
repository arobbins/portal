use leptos::prelude::*;

use super::forms::ExampleFormFields;
use super::StyleguideSection;
use crate::components::{Button, ButtonType, Form, Modal};

#[component]
pub fn ModalSection() -> impl IntoView {
    view! {
        <StyleguideSection title="Modal">
            <Button command="show-modal" commandfor="styleguide-modal">"Open modal"</Button>
            <Modal id="styleguide-modal" heading="Update profile">
                <Form>
                    <ExampleFormFields id_prefix="styleguide-modal-form"/>
                    <Button button_type=ButtonType::Submit>"Submit"</Button>
                </Form>
            </Modal>
        </StyleguideSection>
    }
}
