use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Expr, Signal},
    view::{Attributes, Child, View, class, component, view},
};

use super::types::*;
use crate::components::notice::notice;
use crate::components::tone::Tone;

#[component]
pub async fn form(
    cx: &Cx,
    #[default] variant: FormVariant,
    #[default] submitting: Option<&Signal<bool>>,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <form
            class=(class!(variant.class(), attrs.remove("class")))
            if let Some(submitting) = submitting {
                @submit=$(|_e: Event| {
                    submitting.set(true);
                    raw!(
                        "window.addEventListener('pageshow', (event) => {
                        if (event.persisted) location.reload();
                    }, { once: true })",
                        (),
                    )
                })
            }
            (attrs)
        >
            (child)
        </form>
    })
}

#[component]
pub async fn form_error(
    cx: &Cx,
    #[default] id: Option<&str>,
    #[into]
    #[default(false.into())]
    hidden: Expr<bool>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let class = "a-form-error";
    let hidden_class = "a-form-error a-form-error--is-hidden";

    Ok(view! {
        <div
            id=(id.unwrap_or(FORM_ERROR_ID))
            role="alert"
            :class=$(if hidden { hidden_class } else { class })
        >
            notice(tone: Tone::Error, (child))
        </div>
    })
}

#[component]
pub async fn form_actions(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div class="a-form-actions">(child)</div> })
}

#[component]
pub async fn form_group(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div class="a-form-group">(child)</div> })
}

#[component]
pub async fn form_row(cx: &Cx, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div class="a-form-row">(child)</div> })
}

#[component]
pub async fn form_col(
    cx: &Cx,
    #[default] half: bool,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let class = if half {
        "a-form-col a-form-col--half"
    } else {
        "a-form-col"
    };

    Ok(view! { <div class=(class)>(child)</div> })
}

#[component]
pub async fn form_label(
    cx: &Cx,
    for_id: &str,
    #[default] hidden: bool,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let class = if hidden {
        "u-visually-hidden"
    } else {
        FORM_LABEL_CLASS
    };
    Ok(view! { <label class=(class) for=(for_id)>(child)</label> })
}
