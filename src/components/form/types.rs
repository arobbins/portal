pub const FORM_ERROR_ID: &str = "form-error";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InputType {
    #[default]
    Text,
    Email,
    Password,
}

impl InputType {
    pub(super) fn as_str(self) -> &'static str {
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
    OneTimeCode,
}

impl Autocomplete {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Email => "email",
            Self::CurrentPassword => "current-password",
            Self::NewPassword => "new-password",
            Self::UsernameWebauthn => "username webauthn",
            Self::OneTimeCode => "one-time-code",
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
    pub(super) fn class(self) -> &'static str {
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
pub struct LabelLink<'a> {
    pub href: &'a str,
    pub text: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChoiceKind {
    Radio,
    Checkbox,
}

impl ChoiceKind {
    pub(super) fn input_type(self) -> &'static str {
        match self {
            Self::Radio => "radio",
            Self::Checkbox => "checkbox",
        }
    }
}

pub(super) fn invalid_attr(invalid: bool) -> Option<&'static str> {
    invalid.then_some("true")
}

pub(super) fn error_link(invalid: bool) -> Option<&'static str> {
    invalid.then_some(FORM_ERROR_ID)
}

pub(super) const FORM_LABEL_CLASS: &str = "a-label a-label--md a-form-label";
pub(super) const FORM_LEGEND_CLASS: &str = "a-label a-label--md a-form-legend";
