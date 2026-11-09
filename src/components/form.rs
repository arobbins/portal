use topcoat::{
    Result,
    runtime::{Event, Signal},
    view::{Attributes, Child, View, class, component, view},
};

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

#[component]
pub async fn form(
    #[default] submitting: Option<&Signal<bool>>,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <form
            class=(class!("a-form", attrs.remove("class")))
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
pub async fn form_group(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <fieldset class="a-form-group">(child)</fieldset> })
}

#[component]
pub async fn form_row(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <div class="a-form-row">(child)</div> })
}

#[component]
pub async fn form_col(#[default] half: bool, #[default] child: Child<'_>) -> Result<impl View> {
    let class = if half {
        "a-form-col a-form-col--half"
    } else {
        "a-form-col"
    };

    Ok(view! { <div class=(class)>(child)</div> })
}

#[component]
pub async fn form_label(for_id: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! { <label class="a-form-label" for=(for_id)>(child)</label> })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Autocomplete {
    Name,
    Email,
    CurrentPassword,
    NewPassword,
}

impl Autocomplete {
    fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Email => "email",
            Self::CurrentPassword => "current-password",
            Self::NewPassword => "new-password",
        }
    }
}

#[component]
pub async fn form_input(
    id: &str,
    name: &str,
    #[default] input_type: InputType,
    #[default] placeholder: Option<&str>,
    #[default] value: Option<&str>,
    #[default] required: bool,
    #[default] autocomplete: Option<Autocomplete>,
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
            (attrs)
        >
    })
}

#[component]
pub async fn form_field(
    label: &str,
    id: &str,
    name: &str,
    #[default] input_type: InputType,
    #[default] placeholder: Option<&str>,
    #[default] value: Option<&str>,
    #[default] required: bool,
    #[default] autocomplete: Option<Autocomplete>,
    #[default] input_attrs: Attributes,
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
                attrs: input_attrs
            )
        )
    })
}
