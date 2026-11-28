use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Event, Signal, signal},
    view::{View, component, view},
};

use super::export_routes::{export_data_js, export_data_submit};
use crate::account::tabs::AccountSection;
use crate::auth::pages::reauth_field::reauth_field;
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::form::{
    DialogForm, ModalSubmit, ModalSuccess, dialog_input, dialog_opener, form_group, modal_form,
};
use crate::components::property_list::property;
use crate::components::toaster::NO_TOAST;

const EXPORT_MODAL: &str = "export-data-dialog";
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
pub async fn export_row(cx: &Cx, form: &DialogForm, drafts: &ExportDrafts) -> Result<impl View> {
    let ExportDrafts { blank, password } = drafts;
    let section = AccountSection::ExportData;

    Ok(view! {
        property(
            name: section.label(),
            id: Some(section.anchor()),
            description: Some(
                "Download a copy of everything we keep about you, as spreadsheet (CSV) files in one ZIP.",
            ),
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                attrs: dialog_opener(cx, form, EXPORT_MODAL, &[blank, password]),
                "Export data"
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
    let submit = ModalSubmit {
        procedure: export_data_js,
        action: href!(export_data_submit).resolve(cx),
        label: "Export data",
        success_toast: NO_TOAST,
        unreachable: UNREACHABLE,
    };

    Ok(view! {
        modal_form(
            id: EXPORT_MODAL,
            heading: "Export data",
            form: form,
            toast: toast,
            submit: submit,
            args: $((password.get(),)),
            on_success: ModalSuccess::DownloadFile,
            clear: Some((blank, password)),
            export_fields(form: form, drafts: drafts)
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
