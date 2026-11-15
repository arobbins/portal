use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, View, component, view},
};

use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::auth::session::current_user;
use crate::components::form::{
    Autocomplete, InputType, form_error, form_group, form_input, form_label,
};
use crate::components::notice::notice;
use crate::components::tone::Tone;
use crate::passkeys::actions::has_passkey;
use crate::web::env;

const LABEL: &str = "Current password";

#[component]
pub async fn reauth_field(cx: &Cx, id: &str, #[default] attrs: Attributes) -> Result<impl View> {
    let error_id = format!("{id}-passkey-error");
    let has_passkey = match current_user(cx) {
        Some(user) => has_passkey(&env(cx), user.id).await?,
        None => false,
    };

    Ok(view! {
        <div data-reauth="">
            <div data-reauth-password="">
                form_group(
                    if has_passkey {
                        <div class="a-form-label-row">
                            form_label(for_id: id, (LABEL))
                            <button
                                type="button"
                                class="a-form-label-row__link"
                                data-passkey-confirm=""
                                data-passkey-only=""
                            >
                                "Use a passkey instead"
                            </button>
                        </div>
                    } else {
                        form_label(for_id: id, (LABEL))
                    }
                    form_input(
                        id: id,
                        name: CURRENT_PASSWORD_FIELD,
                        input_type: InputType::Password,
                        required: true,
                        autocomplete: Some(Autocomplete::CurrentPassword),
                        attrs: attrs
                    )
                )
            </div>
            if has_passkey {
                <div class="a-form-reauth-option" data-reauth-done="" hidden="">
                    notice(tone: Tone::Success, "Confirmed with your passkey.")
                </div>
                form_error(id: Some(&error_id), hidden: true, "")
            }
        </div>
    })
}
