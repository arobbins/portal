use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Expr, Signal},
    view::{Attributes, Child, View, class, component, view},
};

use super::notice::{NoticeKind, notice};

pub const FORM_ERROR_ID: &str = "form-error";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Autocomplete {
    Name,
    Email,
    CurrentPassword,
    NewPassword,
    UsernameWebauthn,
}

impl Autocomplete {
    fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Email => "email",
            Self::CurrentPassword => "current-password",
            Self::NewPassword => "new-password",
            Self::UsernameWebauthn => "username webauthn",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FormVariant {
    #[default]
    Inline,
    Page,
}

impl FormVariant {
    fn class(self) -> &'static str {
        match self {
            Self::Inline => "a-form",
            Self::Page => "a-form a-form--page",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice<'a> {
    pub value: &'a str,
    pub label: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChoiceKind {
    Radio,
    Checkbox,
}

impl ChoiceKind {
    fn input_type(self) -> &'static str {
        match self {
            Self::Radio => "radio",
            Self::Checkbox => "checkbox",
        }
    }
}

fn invalid_attr(invalid: bool) -> Option<&'static str> {
    invalid.then_some("true")
}

fn error_link(invalid: bool) -> Option<&'static str> {
    invalid.then_some(FORM_ERROR_ID)
}

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
            notice(kind: NoticeKind::Error, (child))
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
pub async fn form_label(cx: &Cx, for_id: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <label class="a-form-label" for=(for_id)>(child)</label> })
}

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
    #[default] attrs: Attributes,
) -> Result<impl View> {
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
    #[default] attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        form_group(
            form_label(for_id: id, (label))
            form_input(
                id: id,
                name: name,
                input_type: input_type,
                placeholder: placeholder,
                value: value,
                required: required,
                autocomplete: autocomplete,
                invalid: invalid,
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
            <legend class="a-form-legend">(legend)</legend>
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
            required: required
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
) -> Result<impl View> {
    Ok(view! {
        choice_group(
            kind: ChoiceKind::Checkbox,
            legend: legend,
            id: id,
            name: name,
            options: options,
            checked: checked,
            required: false
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
