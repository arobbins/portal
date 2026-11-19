use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use super::export_routes::{export_data_js, export_data_submit};
use crate::account::tabs::AccountSection;
use crate::action::FieldError;
use crate::auth::pages::reauth_field::reauth_field;
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{
    DialogForm, dialog_form_error, dialog_input, form, form_actions, form_group,
};
use crate::components::modal::modal;
use crate::components::toaster::NO_TOAST;

pub const EXPORT_MODAL: &str = "export-data-dialog";
const UNREACHABLE: &str = "We couldn't prepare your data. Please try again.";

pub struct ExportDrafts {
    blank: Signal<String>,
    password: Signal<String>,
}

impl ExportDrafts {
    pub fn new(cx: &Cx) -> Self {
        Self {
            blank: signal(cx, String::new),
            password: signal(cx, String::new),
        }
    }
}

#[component]
pub async fn export_card(cx: &Cx, form: &DialogForm, drafts: &ExportDrafts) -> Result<impl View> {
    let ExportDrafts { blank, password } = drafts;
    let DialogForm {
        error_field,
        error_message,
        ..
    } = form;
    let section = AccountSection::ExportData;

    Ok(view! {
        card(
            id: Some(section.anchor()),
            card_header(
                title: section.label(),
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        command="show-modal"
                        commandfor=(EXPORT_MODAL)
                        @click=$(|_e: Event| {
                            blank.set("".to_owned());
                            password.set("".to_owned());
                            error_field.set("".to_owned());
                            error_message.set("".to_owned());
                        })
                    },
                    "Export data"
                )
            )
            card_body(
                <p>
                    "Download a copy of everything we keep about you, as spreadsheet (CSV) files in one ZIP."
                </p>
            )
        )
    })
}

#[component]
pub async fn export_dialog(
    cx: &Cx,
    form: &DialogForm,
    drafts: &ExportDrafts,
    toast: &Signal<String>,
) -> Result<impl View> {
    let ExportDrafts { blank, password } = drafts;
    let DialogForm {
        error_field,
        error_message,
        is_submitting,
    } = form;
    let modal_id = EXPORT_MODAL.to_string();
    let export_failed: std::result::Result<String, FieldError> =
        Err(FieldError::general(UNREACHABLE));

    Ok(view! {
        modal(
            id: EXPORT_MODAL,
            heading: "Export data",
            form(
                attrs: attributes! {
                    method="post"
                    action=(href!(export_data_submit))
                    @submit=$(async |e: Event| {
                        e.prevent_default();
                        is_submitting.set(true);
                        error_field.set("".to_owned());
                        error_message.set("".to_owned());
                        toast.set(NO_TOAST.to_owned());
                        let _pending = export_data_js(password.get());
                        let result = raw!(
                            "await Promise.resolve(${_pending}).catch(() => ${export_failed})",
                            export_failed.clone(),
                        );
                        is_submitting.set(false);
                        if result.is_ok() {
                            blank.set("".to_owned());
                            password.set("".to_owned());
                            let _file = result.unwrap();
                            raw!(
                                "{ const [name, data] = String(${_file}).split('\\n'); const link = document.createElement('a'); link.href = 'data:application/zip;base64,' + data; link.download = name; document.body.append(link); link.click(); link.remove(); document.getElementById(String(${modal_id})).close(); }",
                                (),
                            );
                        } else {
                            let failure = result.unwrap_err();
                            error_field.set(failure.field);
                            error_message.set(failure.message);
                        }
                    })
                },
                export_fields(form: form, drafts: drafts)
                dialog_form_error(form: form)
                form_actions(
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Export data"
                    )
                )
            )
        )
    })
}

#[component]
async fn export_fields(cx: &Cx, form: &DialogForm, drafts: &ExportDrafts) -> Result<impl View> {
    let ExportDrafts { blank, password } = drafts;

    Ok(view! {
        form_group(
            <p>
                "Confirm it's you. The file holds your sign-in history and IP addresses, so keep it somewhere safe."
            </p>
        )
        reauth_field(
            id: "export-data-password",
            suggest_passkey: false,
            attrs: dialog_input(cx, form, CURRENT_PASSWORD_FIELD, blank, password)
        )
    })
}

#[component]
pub async fn export_first_prompt(cx: &Cx, from_modal: &str) -> Result<impl View> {
    let from_modal = from_modal.to_string();
    let export_modal = EXPORT_MODAL.to_string();

    Ok(view! {
        form_group(
            <p>
                "Want a copy first? "
                <a
                    href=(AccountSection::ExportData.path())
                    @click=$(|e: Event| {
                        e.prevent_default();
                        raw!(
                            "document.getElementById(String(${from_modal})).close(); document.getElementById(String(${export_modal})).showModal()",
                            (),
                        );
                    })
                >
                    "Export your data"
                </a>
            </p>
        )
    })
}
