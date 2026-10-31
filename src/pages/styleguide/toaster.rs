use leptos::prelude::*;

use super::StyleguideSection;
use crate::components::{dispatch_toast, Button, NoticeKind, Toaster};

#[component]
pub fn ToasterSection() -> impl IntoView {
    view! {
        <StyleguideSection title="Toaster">
            <ShowToastButton/>
            <Toaster kind=NoticeKind::Success>"Success! This is a toaster notice."</Toaster>
        </StyleguideSection>
    }
}

#[island]
fn ShowToastButton() -> impl IntoView {
    view! { <Button on:click=move |_| dispatch_toast("toast")>"Show toast"</Button> }
}
