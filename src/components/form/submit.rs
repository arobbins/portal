use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Expr, Signal, Surrogated, TypedProcedure, signal},
    view::{Child, View, attributes, component, view},
};

use super::dialog::{DialogForm, dialog_form_error};
use super::layout::{form, form_actions};
use crate::action::FieldError;
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::modal::{modal, modal_cancel};
use crate::components::toaster::NO_TOAST;

pub struct ModalSubmit<'a, P: TypedProcedure> {
    pub procedure: P,
    pub action: String,
    pub label: &'a str,
    pub success_toast: &'a str,
    pub unreachable: &'a str,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ModalSuccess<'a> {
    #[default]
    Close,
    Navigate(&'a str),
    NavigateToResult,
    DownloadFile,
    KeepOpen,
}

impl ModalSuccess<'_> {
    fn mode(self) -> &'static str {
        match self {
            Self::Close => "close",
            Self::Navigate(_) => "navigate",
            Self::NavigateToResult => "navigate_to_result",
            Self::DownloadFile => "download_file",
            Self::KeepOpen => "keep_open",
        }
    }

    fn target(self) -> String {
        match self {
            Self::Navigate(path) => path.to_string(),
            _ => String::new(),
        }
    }
}

pub struct SubmitSettings<'a> {
    pub modal_id: &'a str,
    pub on_success: ModalSuccess<'a>,
    pub saved: Option<&'a Signal<String>>,
    pub clear: Option<(&'a Signal<String>, &'a Signal<String>)>,
}

#[component]
pub async fn modal_form<P>(
    cx: &Cx,
    id: &str,
    heading: &str,
    form: &DialogForm,
    toast: &Signal<String>,
    submit: ModalSubmit<'_, P>,
    args: Expr<P::Args>,
    #[default] saved: Option<&Signal<String>>,
    #[default] variant: ButtonVariant,
    #[default] on_success: ModalSuccess<'_>,
    #[default] clear: Option<(&Signal<String>, &Signal<String>)>,
    #[default] child: Child<'_>,
) -> Result<impl View>
where
    P: TypedProcedure<Output = std::result::Result<String, FieldError>> + Surrogated + Copy,
    P::Args: Clone + Send + Sync + 'static,
    <P as Surrogated>::Surrogate: serde::Serialize,
{
    let settings = SubmitSettings {
        modal_id: id,
        on_success,
        saved,
        clear,
    };

    Ok(view! {
        modal(
            id: id,
            heading: heading,
            dialog_submit_form(
                form: form,
                toast: toast,
                submit: submit,
                args: args,
                settings: settings,
                variant: variant,
                (child)
            )
        )
    })
}

#[component]
pub async fn dialog_submit_form<P>(
    cx: &Cx,
    form: &DialogForm,
    toast: &Signal<String>,
    submit: ModalSubmit<'_, P>,
    args: Expr<P::Args>,
    settings: SubmitSettings<'_>,
    #[default] variant: ButtonVariant,
    #[default] child: Child<'_>,
) -> Result<impl View>
where
    P: TypedProcedure<Output = std::result::Result<String, FieldError>> + Surrogated + Copy,
    P::Args: Clone + Send + Sync + 'static,
    <P as Surrogated>::Surrogate: serde::Serialize,
{
    let (error_field, error_message) = (&form.error_field, &form.error_message);
    let is_submitting = &form.is_submitting;
    let ModalSubmit {
        procedure,
        action,
        label,
        success_toast,
        unreachable,
    } = submit;
    let (saved, blank, password) = success_signals(cx, &settings);
    let success_toast = success_toast.to_string();
    let (modal_id, mode, target) = settings.after_success();
    let failed: std::result::Result<String, FieldError> = Err(FieldError::general(unreachable));

    Ok(view! {
        form(
            attrs: attributes! {
                method="post"
                action=(&action)
                @submit=$(async |e: Event| {
                    e.prevent_default();
                    is_submitting.set(true);
                    error_field.set("".to_owned());
                    error_message.set("".to_owned());
                    toast.set(NO_TOAST.to_owned());
                    let _values = args;
                    let outcome = raw!(
                        "await Promise.resolve(${procedure}.call(...${_values})).catch(() => ${failed})",
                        failed.clone(),
                    );
                    is_submitting.set(false);
                    if outcome.is_ok() {
                        let _value = outcome.unwrap();
                        saved.set(_value.clone());
                        toast.set(success_toast.clone());
                        blank.set("".to_owned());
                        password.set("".to_owned());
                        raw!(
                            "{ const mode = String(${mode}); const value = String(${_value}); const dialog = document.getElementById(String(${modal_id})); if (mode === 'navigate') { location.assign(String(${target})); } else if (mode === 'navigate_to_result') { location.assign(value); } else if (mode === 'download_file') { const [name, data] = value.split('\\n'); const link = document.createElement('a'); link.href = 'data:application/zip;base64,' + data; link.download = name; document.body.append(link); link.click(); link.remove(); dialog.close(); } else if (mode === 'close') { dialog.close(); } }",
                            (),
                        );
                    } else {
                        let failure = outcome.unwrap_err();
                        error_field.set(failure.field);
                        error_message.set(failure.message);
                    }
                })
            },
            (child)
            modal_form_footer(
                form: form,
                id: settings.modal_id,
                label: label,
                variant: variant
            )
        )
    })
}

fn success_signals(
    cx: &Cx,
    settings: &SubmitSettings<'_>,
) -> (Signal<String>, Signal<String>, Signal<String>) {
    let fresh = || signal(cx, String::new);
    let saved = settings.saved.cloned().unwrap_or_else(fresh);
    let (blank, password) = match settings.clear {
        Some((blank, password)) => (blank.clone(), password.clone()),
        None => (fresh(), fresh()),
    };
    (saved, blank, password)
}

impl SubmitSettings<'_> {
    fn after_success(&self) -> (String, String, String) {
        (
            self.modal_id.to_string(),
            self.on_success.mode().to_string(),
            self.on_success.target(),
        )
    }
}

#[component]
async fn modal_form_footer(
    cx: &Cx,
    form: &DialogForm,
    id: &str,
    label: &str,
    variant: ButtonVariant,
) -> Result<impl View> {
    let is_submitting = &form.is_submitting;

    Ok(view! {
        dialog_form_error(form: form)
        form_actions(
            modal_cancel(modal_id: id)
            button(
                variant: variant,
                button_type: ButtonType::Submit,
                busy: $(is_submitting.get()),
                (label)
            )
        )
    })
}
