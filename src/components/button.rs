use topcoat::{
    Result,
    context::Cx,
    runtime::Expr,
    view::{Attributes, Child, View, component, view},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
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
pub async fn button(
    cx: &Cx,
    #[default] variant: ButtonVariant,
    #[default] size: ButtonSize,
    #[default] button_type: ButtonType,
    #[into]
    #[default(false.into())]
    busy: Expr<bool>,
    #[into]
    #[default(false.into())]
    disabled: Expr<bool>,
    #[default] attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let class = format!("a-button {} {}", variant.modifier(), size.modifier());
    let busy_class = format!("{class} a-button--is-busy");

    Ok(view! {
        <button
            type=(button_type.as_str())
            :class=$(if busy { busy_class.clone() } else { class.clone() })
            :disabled=$(if busy { true } else { disabled })
            (attrs)
        >
            (child)
        </button>
    })
}
