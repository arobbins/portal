use topcoat::{
    Result,
    context::Cx,
    view::{Attributes, View, component, view},
};

use crate::account::tabs::AccountSection;
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::auth::session::current_user;
use crate::components::form::{
    Autocomplete, InputType, form_error, form_group, form_input, form_label,
};
use crate::components::notice::notice;
use crate::components::tone::Tone;
use crate::passkeys::actions::has_passkey;
use crate::web::env;

const PASSWORD_LABEL: &str = "Current password";

#[component]
pub async fn reauth_field(
    cx: &Cx,
    id: &str,
    #[default(true)] suggest_passkey: bool,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    let has_passkey = match current_user(cx) {
        Some(user) => has_passkey(&env(cx), user.id).await?,
        None => false,
    };
    let add_passkey = AccountSection::Passkeys.path();

    Ok(view! {
        <div
            data-reauth=""
            data-reauth-has-passkey=(if has_passkey { "true" } else { "false" })
        >
            if has_passkey {
                passkey_panel(id: id)
            }
            <div data-reauth-password="">
                form_group(
                    <div class="a-form-label-row">
                        form_label(for_id: id, (PASSWORD_LABEL))
                        if has_passkey {
                            <button
                                type="button"
                                class="a-form-label-row__link"
                                data-reauth-use-passkey=""
                                data-passkey-only=""
                            >
                                "Use a passkey instead"
                            </button>
                        } else if suggest_passkey {
                            <a
                                class="a-form-label-row__link"
                                href=(&add_passkey)
                                data-passkey-only=""
                            >
                                "Add a passkey for faster authentication"
                            </a>
                        }
                    </div>
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
        </div>
    })
}

#[component]
async fn passkey_panel(cx: &Cx, id: &str) -> Result<impl View> {
    let error_id = format!("{id}-passkey-error");

    Ok(view! {
        <div data-reauth-passkey="" hidden="">
            form_group(
                <div class="a-form-label-row">
                    <span class="a-label a-label--md a-form-label">
                        "Confirm it's you"
                    </span>
                    <button
                        type="button"
                        class="a-form-label-row__link"
                        data-reauth-use-password=""
                    >
                        "Use your password instead"
                    </button>
                </div>
                notice(
                    tone: Tone::Info,
                    "We'll ask for your passkey when you continue."
                )
            )
            form_error(id: Some(&error_id), hidden: true, "")
        </div>
    })
}
