use std::convert::Infallible;

use crate::action::{ActionResult, Caller};
use crate::api::catalog::{ErrorDoc, HttpMethod};
use crate::api::registry::{ApiAction, NoInput};
use crate::env::Env;
use crate::permission::Permission;
use crate::preferences::actions::{
    NewsletterSettings, UpdateNewsletters, get_newsletters, update_newsletters,
};

pub struct GetNewslettersAction;

impl ApiAction for GetNewslettersAction {
    const ID: &'static str = "get-newsletters";
    const GROUP: &'static str = "Newsletters";
    const METHOD: HttpMethod = HttpMethod::Get;
    const PATH: &'static str = "/api/v1/newsletters";
    const SUMMARY: &'static str = "Get your newsletter choices";
    const DESCRIPTION: &'static str = "Returns whether the account is subscribed to the General newsletter and to Product updates.";
    const PERMISSION: Permission = Permission::ViewProfile;

    type Input = NoInput;
    type Output = NewsletterSettings;
    type Problem = Infallible;

    async fn run(
        env: &Env<'_>,
        caller: &Caller,
        _input: &NoInput,
    ) -> ActionResult<NewsletterSettings, Infallible> {
        get_newsletters(env, caller).await
    }

    fn example_input() -> Option<NoInput> {
        None
    }

    fn example_output() -> NewsletterSettings {
        NewsletterSettings {
            general_newsletter: true,
            product_updates: true,
        }
    }

    fn problems() -> Vec<ErrorDoc> {
        Vec::new()
    }
}

pub struct UpdateNewslettersAction;

impl ApiAction for UpdateNewslettersAction {
    const ID: &'static str = "update-newsletters";
    const GROUP: &'static str = "Newsletters";
    const METHOD: HttpMethod = HttpMethod::Patch;
    const PATH: &'static str = "/api/v1/newsletters";
    const SUMMARY: &'static str = "Change your newsletter choices";
    const DESCRIPTION: &'static str = "Subscribes to or unsubscribes from either newsletter or both; a field left out keeps its choice. Returns both choices. Each real change appears in the app straight away and is recorded in the account's activity log; saving a choice that didn't change records nothing.";
    const PERMISSION: Permission = Permission::EditProfile;

    type Input = UpdateNewsletters;
    type Output = NewsletterSettings;
    type Problem = Infallible;

    async fn run(
        env: &Env<'_>,
        caller: &Caller,
        input: &UpdateNewsletters,
    ) -> ActionResult<NewsletterSettings, Infallible> {
        update_newsletters(env, caller, input).await
    }

    fn example_input() -> Option<UpdateNewsletters> {
        Some(UpdateNewsletters {
            general_newsletter: None,
            product_updates: Some(false),
        })
    }

    fn example_output() -> NewsletterSettings {
        NewsletterSettings {
            general_newsletter: true,
            product_updates: false,
        }
    }

    fn problems() -> Vec<ErrorDoc> {
        Vec::new()
    }
}
