use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputType {
    #[default]
    Text,
    Email,
    Password,
}

impl InputType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Email => "email",
            Self::Password => "password",
        }
    }
}

#[component]
pub fn Form(children: Children) -> impl IntoView {
    view! { <form class="a-form">{children()}</form> }
}

#[component]
pub fn FormGroup(children: Children) -> impl IntoView {
    view! { <fieldset class="a-form-group">{children()}</fieldset> }
}

#[component]
pub fn FormRow(children: Children) -> impl IntoView {
    view! { <div class="a-form-row">{children()}</div> }
}

#[component]
pub fn FormCol(#[prop(optional)] half: bool, children: Children) -> impl IntoView {
    let class = if half { "a-form-col a-form-col--half" } else { "a-form-col" };

    view! { <div class=class>{children()}</div> }
}

#[component]
pub fn FormLabel(#[prop(into)] for_id: String, children: Children) -> impl IntoView {
    view! { <label class="a-form-label" for=for_id>{children()}</label> }
}

#[component]
pub fn FormInput(
    #[prop(into)] id: String,
    #[prop(into)] name: String,
    #[prop(optional)] input_type: InputType,
    #[prop(into, optional)] placeholder: MaybeProp<String>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] required: bool,
) -> impl IntoView {
    view! {
        <input
            class="a-form-input"
            id=id
            name=name
            type=input_type.as_str()
            placeholder=move || placeholder.get()
            value=move || value.get()
            required=required
        />
    }
}

#[component]
pub fn FormField(
    #[prop(into)] label: String,
    #[prop(into)] id: String,
    #[prop(into)] name: String,
    #[prop(optional)] input_type: InputType,
    #[prop(into, optional)] placeholder: MaybeProp<String>,
    #[prop(into, optional)] value: MaybeProp<String>,
    #[prop(optional)] required: bool,
) -> impl IntoView {
    view! {
        <FormGroup>
            <FormLabel for_id=id.clone()>{label}</FormLabel>
            <FormInput
                id=id
                name=name
                input_type=input_type
                placeholder=placeholder
                value=value
                required=required
            />
        </FormGroup>
    }
}
