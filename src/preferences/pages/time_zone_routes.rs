use topcoat::{
    Result,
    context::{Cx, try_context},
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
use crate::preferences::actions::{TimeZoneProblem, UpdateTimeZone, update_time_zone};
use crate::web::{env, redisplay, session};

struct TimeZoneFailed {
    problem: TimeZoneProblem,
}

pub(super) fn time_zone_problem(cx: &Cx) -> Option<TimeZoneProblem> {
    try_context::<TimeZoneFailed>(cx).map(|failed| failed.problem)
}

#[procedure]
pub(super) async fn save_time_zone(
    cx: &Cx,
    time_zone: String,
) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };

    match update_time_zone(&env(cx), &caller, &UpdateTimeZone { time_zone }).await? {
        Ok(saved) => Ok(Ok(saved)),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/time-zone/update")]
pub async fn update_time_zone_submit(
    cx: &Cx,
    Form(input): Form<UpdateTimeZone>,
) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let page = AccountSection::TimeZone.page().path();

    match update_time_zone(&env(cx), &caller, &input).await? {
        Ok(_) => {
            set_flash(&session(cx)?, Flash::TimeZoneSaved).await?;
            Ok(see_other(page))
        }
        Err(error) => {
            let problem = error.for_browser()?;
            Err(redisplay(page, TimeZoneFailed { problem }))
        }
    }
}
