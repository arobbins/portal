use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, View, component, view},
};

use super::layout::{form_group, form_label};
use super::types::*;

#[component]
pub async fn select_field(
    cx: &Cx,
    label: &str,
    id: &str,
    name: &str,
    options: &[Choice<'_>],
    #[default] selected: Option<&str>,
    #[default] placeholder: Option<&str>,
    #[default] required: bool,
    #[default] invalid: bool,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    let options: Vec<(Choice<'_>, bool)> = options
        .iter()
        .map(|option| (*option, selected == Some(option.value)))
        .collect();

    Ok(view! {
        form_group(
            form_label(for_id: id, (label))
            <span class="a-form-select">
                <select
                    class="a-form-input a-form-select__input"
                    id=(id)
                    name=(name)
                    required=(required)
                    aria-invalid=(invalid_attr(invalid))
                    aria-describedby=(error_link(invalid))
                    (attrs)
                >
                    if let Some(placeholder) = placeholder {
                        <option value="" disabled=(true) selected=(selected.is_none())>
                            (placeholder)
                        </option>
                    }
                    #[key(option.value)]
                    for (option, is_selected) in &options {
                        <option value=(option.value) selected=(*is_selected)>
                            (option.label)
                        </option>
                    }
                </select>
            </span>
        )
    })
}

#[component]
async fn choice_group(
    cx: &Cx,
    kind: ChoiceKind,
    legend: &str,
    id: &str,
    name: &str,
    options: &[Choice<'_>],
    checked: &[&str],
    required: bool,
    invalid: bool,
) -> Result<impl View> {
    let options: Vec<(Choice<'_>, String, bool)> = options
        .iter()
        .map(|option| {
            (
                *option,
                format!("{id}-{}", option.value),
                checked.contains(&option.value),
            )
        })
        .collect();

    Ok(view! {
        <fieldset class="a-form-group a-form-fieldset" id=(id)>
            <legend class=(FORM_LEGEND_CLASS)>(legend)</legend>
            #[key(option_id.as_str())]
            for (option, option_id, is_checked) in &options {
                <label class="a-form-choice" for=(option_id.as_str())>
                    <input
                        class="a-form-choice__input"
                        type=(kind.input_type())
                        id=(option_id.as_str())
                        name=(name)
                        value=(option.value)
                        checked=(*is_checked)
                        required=(required)
                        aria-invalid=(invalid_attr(invalid))
                        aria-describedby=(error_link(invalid))
                    >
                    (option.label)
                </label>
            }
        </fieldset>
    })
}

#[component]
pub async fn radio_group(
    cx: &Cx,
    legend: &str,
    id: &str,
    name: &str,
    options: &[Choice<'_>],
    #[default] selected: Option<&str>,
    #[default] required: bool,
    #[default] invalid: bool,
) -> Result<impl View> {
    let checked: Vec<&str> = selected.into_iter().collect();

    Ok(view! {
        choice_group(
            kind: ChoiceKind::Radio,
            legend: legend,
            id: id,
            name: name,
            options: options,
            checked: &checked,
            required: required,
            invalid: invalid
        )
    })
}

#[component]
pub async fn checkbox_group(
    cx: &Cx,
    legend: &str,
    id: &str,
    name: &str,
    options: &[Choice<'_>],
    #[default] checked: &[&str],
    #[default] invalid: bool,
) -> Result<impl View> {
    Ok(view! {
        choice_group(
            kind: ChoiceKind::Checkbox,
            legend: legend,
            id: id,
            name: name,
            options: options,
            checked: checked,
            required: false,
            invalid: invalid
        )
    })
}
