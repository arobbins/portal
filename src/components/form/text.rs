use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, View, component, view},
};

use super::layout::{form_group, form_label};
use super::types::*;

#[component]
pub async fn form_input(
    cx: &Cx,
    id: &str,
    name: &str,
    #[default] input_type: InputType,
    #[default] placeholder: Option<&str>,
    #[default] value: Option<&str>,
    #[default] required: bool,
    #[default] autocomplete: Option<Autocomplete>,
    #[default] invalid: bool,
    #[default] max_length: Option<usize>,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    let max_length = max_length.map(|length| length.to_string());

    Ok(view! {
        <input
            class="a-form-input"
            id=(id)
            name=(name)
            type=(input_type.as_str())
            placeholder=(placeholder)
            value=(value)
            required=(required)
            autocomplete=(autocomplete.map(Autocomplete::as_str))
            maxlength=(max_length)
            aria-invalid=(invalid_attr(invalid))
            aria-describedby=(error_link(invalid))
            (attrs)
        >
    })
}

#[component]
pub async fn form_field(
    cx: &Cx,
    label: &str,
    id: &str,
    name: &str,
    #[default] input_type: InputType,
    #[default] placeholder: Option<&str>,
    #[default] value: Option<&str>,
    #[default] required: bool,
    #[default] autocomplete: Option<Autocomplete>,
    #[default] invalid: bool,
    #[default] label_link: Option<LabelLink<'_>>,
    #[default] hide_label: bool,
    #[default] max_length: Option<usize>,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        form_group(
            if let Some(link) = label_link {
                <div class="a-form-label-row">
                    form_label(for_id: id, (label))
                    <a class="a-form-label-row__link" href=(link.href)>(link.text)</a>
                </div>
            } else {
                form_label(for_id: id, hidden: hide_label, (label))
            }
            form_input(
                id: id,
                name: name,
                input_type: input_type,
                placeholder: placeholder,
                value: value,
                required: required,
                autocomplete: autocomplete,
                invalid: invalid,
                max_length: max_length,
                attrs: attrs
            )
        )
    })
}

#[component]
pub async fn textarea_field(
    cx: &Cx,
    label: &str,
    id: &str,
    name: &str,
    #[default(4)] rows: u32,
    #[default] placeholder: Option<&str>,
    #[default] value: Option<&str>,
    #[default] required: bool,
    #[default] invalid: bool,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        form_group(
            form_label(for_id: id, (label))
            <textarea
                class="a-form-input a-form-input--textarea"
                id=(id)
                name=(name)
                rows=(rows)
                placeholder=(placeholder)
                required=(required)
                aria-invalid=(invalid_attr(invalid))
                aria-describedby=(error_link(invalid))
                (attrs)
            >
                (value)
            </textarea>
        )
    })
}

#[component]
pub async fn code_field(
    cx: &Cx,
    label: &str,
    id: &str,
    name: &str,
    #[default(6)] length: u32,
    #[default] required: bool,
    #[default] invalid: bool,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    let pattern = format!("[0-9]{{{length}}}");

    Ok(view! {
        form_group(
            form_label(for_id: id, (label))
            <input
                class="a-form-input a-form-input--code"
                id=(id)
                name=(name)
                type="text"
                inputmode="numeric"
                autocomplete=(Autocomplete::OneTimeCode.as_str())
                pattern=(pattern.as_str())
                minlength=(length)
                maxlength=(length)
                required=(required)
                aria-invalid=(invalid_attr(invalid))
                aria-describedby=(error_link(invalid))
                (attrs)
            >
        )
    })
}

#[component]
pub async fn checkbox(
    cx: &Cx,
    label: &str,
    id: &str,
    name: &str,
    #[default] checked: bool,
    #[default] required: bool,
    #[default] invalid: bool,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        form_group(
            <label class="a-form-choice" for=(id)>
                <input
                    class="a-form-choice__input"
                    type="checkbox"
                    id=(id)
                    name=(name)
                    checked=(checked)
                    required=(required)
                    aria-invalid=(invalid_attr(invalid))
                    aria-describedby=(error_link(invalid))
                    (attrs)
                >
                (label)
            </label>
        )
    })
}
