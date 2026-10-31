use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
}

impl ButtonVariant {
    fn modifier(self) -> &'static str {
        match self {
            Self::Primary => "a-button--primary",
            Self::Secondary => "a-button--secondary",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl ButtonSize {
    fn modifier(self) -> &'static str {
        match self {
            Self::Sm => "a-button--sm",
            Self::Md => "a-button--md",
            Self::Lg => "a-button--lg",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ButtonType {
    #[default]
    Button,
    Submit,
    Reset,
}

impl ButtonType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Button => "button",
            Self::Submit => "submit",
            Self::Reset => "reset",
        }
    }
}

#[component]
pub fn Button(
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: ButtonSize,
    #[prop(optional)] button_type: ButtonType,
    #[prop(into, optional)] busy: Signal<bool>,
    #[prop(into, optional)] disabled: Signal<bool>,
    #[prop(optional)] command: Option<&'static str>,
    #[prop(into, optional)] commandfor: Option<String>,
    children: Children,
) -> impl IntoView {
    let class = move || {
        let mut class = format!("a-button {} {}", variant.modifier(), size.modifier());
        if busy.get() {
            class.push_str(" a-button--is-busy");
        }
        class
    };

    view! {
        <button
            type=button_type.as_str()
            class=class
            disabled=disabled
            command=command
            commandfor=commandfor
        >
            {children()}
        </button>
    }
}
