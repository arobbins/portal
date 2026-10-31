use leptos::prelude::*;

use super::StyleguideSection;
use crate::components::{Button, ButtonSize, ButtonVariant, Icon, IconName};

#[component]
pub fn ButtonsSection() -> impl IntoView {
    view! {
        <StyleguideSection title="Buttons">
            <Button>"Primary button"</Button>
            <Button busy=true>"Busy button"</Button>
            <Button disabled=true>"Disabled button"</Button>
            <Button size=ButtonSize::Sm>"Small button"</Button>
            <Button size=ButtonSize::Md>"Medium button"</Button>
            <Button size=ButtonSize::Lg>"Large button"</Button>
            <Button variant=ButtonVariant::Secondary size=ButtonSize::Lg>"Secondary button"</Button>
            <Button variant=ButtonVariant::Secondary size=ButtonSize::Md>"Secondary button"</Button>
            <Button variant=ButtonVariant::Secondary size=ButtonSize::Sm>"Secondary button"</Button>
            <Button size=ButtonSize::Sm>
                <Icon name=IconName::Edit/>
                "Button with icon"
            </Button>
        </StyleguideSection>
    }
}
