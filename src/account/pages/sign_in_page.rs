use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::placeholder_page::coming_soon_card;
use crate::account::tabs::AccountSection;
use crate::action::ActionProblem;
use crate::auth::pages::password_card::{PasswordDrafts, change_password_dialog, password_card};
use crate::auth::pages::password_routes::password_failure;
use crate::components::form::{DialogForm, dialog_error_toast, page_toast};
use crate::flash::{Flash, flash_toasts, take_flash};
use crate::passkeys::pages::passkey_routes::{remove_problem, rename_problem};
use crate::passkeys::pages::passkeys_card::passkeys_card;
use crate::two_factor::pages::two_factor_card::two_factor_card;
use crate::two_factor::pages::two_factor_routes::two_factor_failure;

const FLASHES: &[Flash] = &[
    Flash::PasswordChanged,
    Flash::PasskeyAdded,
    Flash::PasskeyRenamed,
    Flash::PasskeyRemoved,
    Flash::TwoFactorDisabled,
];

#[component]
pub async fn sign_in_page(cx: &Cx) -> Result<impl View> {
    let flash = take_flash(cx, FLASHES).await?;
    let password_failure = password_failure(cx);
    let failure = password_failure
        .or_else(|| rename_problem(cx).map(ActionProblem::spec))
        .or_else(|| remove_problem(cx).map(ActionProblem::spec))
        .or_else(|| two_factor_failure(cx).map(|failed| failed.problem.spec()));
    let form = DialogForm::new(cx, password_failure);
    let toast = page_toast(cx, failure.is_some(), flash);
    let drafts = PasswordDrafts::new(cx);

    Ok(view! {
        <div class="p-account__sub-page-content">
            password_card(form: &form, drafts: &drafts)
            passkeys_card()
            two_factor_card()
            coming_soon_card(
                section: AccountSection::SingleSignOn,
                action: "Connect",
                text: "Sign in with your company's identity provider. Coming soon."
            )
            change_password_dialog(form: &form, drafts: &drafts, toast: &toast)
            flash_toasts(toast: &toast, flashes: FLASHES)
            dialog_error_toast(failure: failure, toast: &toast)
        </div>
    })
}
