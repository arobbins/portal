use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Signal, signal},
    view::{View, component, view},
};

use super::admin_access::ADMIN_IDLE_LIMIT;
use super::admin_confirm::admin_confirm_submit;
use super::admin_unlock_routes::unlock_admin_js;
use crate::auth::pages::reauth_field::reauth_field;
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::components::form::{
    DialogForm, ModalSubmit, ModalSuccess, dialog_input, form_group, modal_form,
};
use crate::components::toaster::NO_TOAST;

pub const ADMIN_UNLOCK_MODAL: &str = "admin-unlock";
const UNREACHABLE: &str = "We couldn't open the admin area. Please try again.";

pub struct AdminUnlockDrafts {
    pub blank: Signal<String>,
    pub password: Signal<String>,
    pub next: Signal<String>,
}

impl AdminUnlockDrafts {
    pub fn new(cx: &Cx) -> Self {
        Self {
            blank: signal(cx, String::new),
            password: signal(cx, String::new),
            next: signal(cx, String::new),
        }
    }
}

#[component]
pub async fn admin_unlock_dialog(
    cx: &Cx,
    form: &DialogForm,
    drafts: &AdminUnlockDrafts,
) -> Result<impl View> {
    let AdminUnlockDrafts {
        blank,
        password,
        next,
    } = drafts;
    let intro = format!(
        "Confirm it's you to open the admin area. It locks again after {} minutes without activity.",
        ADMIN_IDLE_LIMIT.whole_minutes()
    );
    let toast = signal(cx, || NO_TOAST.to_string());
    let submit = ModalSubmit {
        procedure: unlock_admin_js,
        action: href!(admin_confirm_submit).resolve(cx),
        label: "Continue",
        success_toast: NO_TOAST,
        unreachable: UNREACHABLE,
    };

    Ok(view! {
        modal_form(
            id: ADMIN_UNLOCK_MODAL,
            heading: "Admin area",
            form: form,
            toast: &toast,
            submit: submit,
            args: $((password.get(), next.get())),
            on_success: ModalSuccess::NavigateToResult,
            form_group(<p>(intro)</p>)
            reauth_field(
                id: "admin-unlock-password",
                suggest_passkey: false,
                attrs: dialog_input(cx, form, CURRENT_PASSWORD_FIELD, blank, password)
            )
        )
    })
}
