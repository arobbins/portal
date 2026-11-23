use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use super::admin_access::ADMIN_IDLE_LIMIT;
use super::admin_confirm::admin_confirm_page;
use super::admin_unlock_routes::unlock_admin_js;
use crate::action::FieldError;
use crate::auth::pages::reauth_field::reauth_field;
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::components::button::{ButtonType, button};
use crate::components::form::{
    DialogForm, dialog_form_error, dialog_input, form, form_actions, form_group,
};
use crate::components::modal::{modal, modal_cancel};

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
    let DialogForm {
        error_field,
        error_message,
        is_submitting,
    } = form;
    let intro = format!(
        "Confirm it's you to open the admin area. It locks again after {} minutes without activity.",
        ADMIN_IDLE_LIMIT.whole_minutes()
    );
    let failed: std::result::Result<String, FieldError> = Err(FieldError::general(UNREACHABLE));

    Ok(view! {
        modal(
            id: ADMIN_UNLOCK_MODAL,
            heading: "Admin area",
            form(
                attrs: attributes! {
                    method="get"
                    action=(href!(admin_confirm_page))
                    @submit=$(async |e: Event| {
                        e.prevent_default();
                        is_submitting.set(true);
                        error_field.set("".to_owned());
                        error_message.set("".to_owned());
                        let _pending = unlock_admin_js(password.get(), next.get());
                        let result = raw!(
                            "await Promise.resolve(${_pending}).catch(() => ${failed})",
                            failed.clone(),
                        );
                        is_submitting.set(false);
                        if result.is_ok() {
                            let _to = result.unwrap();
                            raw!("location.assign(String(${_to}))", ());
                        } else {
                            let failure = result.unwrap_err();
                            error_field.set(failure.field);
                            error_message.set(failure.message);
                        }
                    })
                },
                form_group(<p>(intro)</p>)
                reauth_field(
                    id: "admin-unlock-password",
                    suggest_passkey: false,
                    attrs: dialog_input(
                        cx,
                        form,
                        CURRENT_PASSWORD_FIELD,
                        blank,
                        password,
                    )
                )
                dialog_form_error(form: form)
                form_actions(
                    modal_cancel(modal_id: ADMIN_UNLOCK_MODAL)
                    button(
                        button_type: ButtonType::Submit,
                        busy: $(is_submitting.get()),
                        "Continue"
                    )
                )
            )
        )
    })
}
