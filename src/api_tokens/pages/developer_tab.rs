use topcoat::{
    Result,
    context::{Cx, try_context},
    runtime::signal,
    view::{View, component, view},
};

use super::create_token_dialog::create_token_dialog;
use super::token_fields::TokenDrafts;
use super::token_list::{new_token_card, tokens_card};
use super::token_routes::{CreateTokenFailed, TokenCreated};
use super::token_view::TokenRow;
use crate::action::{ActionProblem, require_browser_caller};
use crate::api_tokens::actions::list_api_tokens;
use crate::components::form::{DialogForm, dialog_error_toast};
use crate::flash::{Flash, flash_toasts, take_flash, toast_name};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};
use crate::web::env;

const FLASHES: &[Flash] = &[Flash::ApiTokenRevoked];
const ERROR_TOAST: &str = "error";

#[component]
pub async fn developer_tab(cx: &Cx) -> Result<impl View> {
    let rows = token_rows(cx).await?;
    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<CreateTokenFailed>(cx).map(|failed| failed.problem);
    let created = try_context::<TokenCreated>(cx).map(|created| created.token.clone());
    let form = DialogForm::new(cx, "", failed.map_or("", ActionProblem::message));
    let initial_toast = if failed.is_some() {
        ERROR_TOAST
    } else {
        toast_name(flash)
    };
    let toast = signal(cx, || initial_toast.to_string());
    let drafts = TokenDrafts::new(cx);

    Ok(view! {
        <div class="p-account__sub-page-content">
            page_heading(text: "Developer", variant: PageHeadingVariant::Compact)
            if let Some(token) = &created {
                new_token_card(token: token)
            }
            tokens_card(rows: &rows, form: &form, drafts: &drafts)
            create_token_dialog(form: &form, drafts: &drafts, toast: &toast)
            flash_toasts(toast: &toast, flashes: FLASHES)
            dialog_error_toast(form: &form, toast: &toast, name: ERROR_TOAST)
        </div>
    })
}

async fn token_rows(cx: &Cx) -> Result<Vec<TokenRow>> {
    let caller = require_browser_caller(cx)?;
    let env = env(cx);
    let tokens = match list_api_tokens(&env, &caller).await? {
        Ok(tokens) => tokens,
        Err(error) => match error.for_browser()? {},
    };
    Ok(tokens
        .iter()
        .map(|token| TokenRow::new(token, env.now()))
        .collect())
}
