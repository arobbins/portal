use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        route,
    },
    runtime::procedure,
};

use crate::account::tabs::AccountSection;
use crate::action::{FieldError, browser_caller, require_browser_caller};
use crate::flash::{Flash, set_flash};
use crate::preferences::actions::{UpdateNewsletters, update_newsletters};
use crate::preferences::newsletters::Newsletter;
use crate::web::{env, session};

#[derive(Debug, Deserialize)]
pub struct NewslettersForm {
    #[serde(default, deserialize_with = "crate::utils::checkbox")]
    general_newsletter: bool,
    #[serde(default, deserialize_with = "crate::utils::checkbox")]
    product_updates: bool,
}

impl From<NewslettersForm> for UpdateNewsletters {
    fn from(form: NewslettersForm) -> Self {
        Self {
            general_newsletter: Some(form.general_newsletter),
            product_updates: Some(form.product_updates),
        }
    }
}

#[procedure]
pub(super) async fn save_general_newsletter(
    cx: &Cx,
    on: bool,
) -> Result<Result<String, FieldError>> {
    save_one(cx, Newsletter::GeneralNewsletter, on).await
}

#[procedure]
pub(super) async fn save_product_updates(cx: &Cx, on: bool) -> Result<Result<String, FieldError>> {
    save_one(cx, Newsletter::ProductUpdates, on).await
}

async fn save_one(cx: &Cx, newsletter: Newsletter, on: bool) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let input = UpdateNewsletters::one(newsletter, on);

    match update_newsletters(&env(cx), &caller, &input).await? {
        Ok(_) => Ok(Ok(String::new())),
        Err(error) => match error.for_browser()? {},
    }
}

#[route(POST "/account/newsletters/update")]
pub async fn update_newsletters_submit(
    cx: &Cx,
    Form(form): Form<NewslettersForm>,
) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    match update_newsletters(&env(cx), &caller, &form.into()).await? {
        Ok(_) => {
            set_flash(&session(cx)?, Flash::NewslettersSaved).await?;
            Ok(see_other(AccountSection::Newsletters.path()))
        }
        Err(error) => match error.for_browser()? {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_no_js_form_saves_both_choices_and_an_unticked_box_is_off() {
        let ticked: NewslettersForm = serde_urlencoded::from_str("general_newsletter=on").unwrap();
        let input = UpdateNewsletters::from(ticked);
        assert_eq!(input.general_newsletter, Some(true));
        assert_eq!(input.product_updates, Some(false));

        let none: NewslettersForm = serde_urlencoded::from_str("").unwrap();
        let input = UpdateNewsletters::from(none);
        assert_eq!(input.general_newsletter, Some(false));
        assert_eq!(input.product_updates, Some(false));
    }
}
