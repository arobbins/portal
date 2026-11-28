use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Signal, signal},
    view::{Attributes, View, attributes, component, view},
};

use super::layout::form_error;
use super::types::FORM_ERROR_ID;
use crate::action::ProblemSpec;
use crate::components::toaster::toaster;
use crate::components::tone::Tone;
use crate::flash::{Flash, flash_toast};

const DIALOG_ERROR_TOAST: &str = "error";

pub struct DialogForm {
    pub error_field: Signal<String>,
    pub error_message: Signal<String>,
    pub is_submitting: Signal<bool>,
}

impl DialogForm {
    pub fn new(cx: &Cx, failure: Option<ProblemSpec>) -> Self {
        let field = failure.and_then(|spec| spec.field).unwrap_or_default();
        let message = failure.map_or("", |spec| spec.message);
        let (field, message) = (field.to_string(), message.to_string());
        Self {
            error_field: signal(cx, move || field),
            error_message: signal(cx, move || message),
            is_submitting: signal(cx, || false),
        }
    }
}

pub fn page_toast(cx: &Cx, form_failed: bool, flash: Option<Flash>) -> Signal<String> {
    if form_failed {
        signal(cx, || DIALOG_ERROR_TOAST.to_string())
    } else {
        flash_toast(cx, flash)
    }
}

pub fn dialog_opener(
    cx: &Cx,
    form: &DialogForm,
    modal_id: &str,
    clear: &[&Signal<String>],
) -> Attributes {
    let DialogForm {
        error_field,
        error_message,
        ..
    } = form;
    debug_assert!(clear.len() <= 4, "dialog_opener clears at most four drafts");
    let slot = |index: usize| match clear.get(index) {
        Some(draft) => (*draft).clone(),
        None => signal(cx, String::new),
    };
    let (first, second, third, fourth) = (slot(0), slot(1), slot(2), slot(3));

    attributes! {
        command="show-modal"
        commandfor=(modal_id)
        @click=$(|_e: Event| {
            first.set("".to_owned());
            second.set("".to_owned());
            third.set("".to_owned());
            fourth.set("".to_owned());
            error_field.set("".to_owned());
            error_message.set("".to_owned());
        })
    }
}

pub fn dialog_input(
    cx: &Cx,
    form: &DialogForm,
    field: &str,
    value: &Signal<String>,
    draft: &Signal<String>,
) -> Attributes {
    let error_field = &form.error_field;
    let field = field.to_string();

    attributes! {
        :value=$(value.get())
        :aria-invalid=$(if error_field.get() == field { "true" } else { "false" })
        aria-describedby=(FORM_ERROR_ID)
        @input=$(|e: Event| draft.set(e.target.value))
    }
}

#[component]
pub async fn dialog_form_error(cx: &Cx, form: &DialogForm) -> Result<impl View> {
    let error_message = &form.error_message;

    Ok(view! {
        form_error(hidden: $(error_message.get().is_empty()), $(error_message.get()))
    })
}

#[component]
pub async fn dialog_error_toast(
    cx: &Cx,
    failure: Option<ProblemSpec>,
    toast: &Signal<String>,
) -> Result<impl View> {
    let message = failure.map_or("", |spec| spec.message);

    Ok(view! {
        toaster(toast: toast, name: DIALOG_ERROR_TOAST, tone: Tone::Error, (message))
    })
}
