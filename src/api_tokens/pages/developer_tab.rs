use topcoat::{
    Result,
    context::{Cx, try_context},
    view::{View, component, view},
};

use super::create_token_dialog::create_token_dialog;
use super::token_fields::TokenDrafts;
use super::token_list::{new_token_card, tokens_card};
use super::token_routes::{CreateTokenFailed, TokenCreated};
use super::token_view::TokenRow;
use crate::action::{ActionProblem, ForPage, require_browser_caller};
use crate::api_tokens::actions::list_api_tokens;
use crate::components::form::{DialogForm, dialog_error_toast, page_toast};
use crate::flash::{Flash, flash_toasts, take_flash};
use crate::oauth::pages::connected_apps_card::connected_apps_card;
use crate::web::env;

const FLASHES: &[Flash] = &[Flash::ApiTokenRevoked, Flash::AppDisconnected];

#[component]
pub async fn developer_tab(cx: &Cx) -> Result<impl View> {
    let rows = token_rows(cx).await?;
    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<CreateTokenFailed>(cx).map(|failed| failed.problem);
    let created = try_context::<TokenCreated>(cx).map(|created| created.token.expose().to_string());
    let failure = failed.map(ActionProblem::spec);
    let form = DialogForm::new(cx, failure);
    let toast = page_toast(cx, failed.is_some(), flash);
    let drafts = TokenDrafts::new(cx);

    Ok(view! {
        <div class="p-account__sub-page-content">
            if let Some(token) = &created {
                new_token_card(token: token)
            }
            tokens_card(rows: &rows, form: &form, drafts: &drafts)
            connected_apps_card()
            create_token_dialog(form: &form, drafts: &drafts, toast: &toast)
            flash_toasts(toast: &toast, flashes: FLASHES)
            dialog_error_toast(failure: failure, toast: &toast)
        </div>
    })
}

async fn token_rows(cx: &Cx) -> Result<Vec<TokenRow>> {
    let caller = require_browser_caller(cx)?;
    let env = env(cx);
    let tokens = list_api_tokens(&env, &caller).await?.for_page()?;
    let zone = caller.user.zone();
    Ok(tokens
        .iter()
        .map(|token| TokenRow::new(token, env.now(), &zone))
        .collect())
}
