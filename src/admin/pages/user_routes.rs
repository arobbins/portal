use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, route,
    },
};

use super::admin_access::require_admin;
use super::user_page::{admin_user_page, id};
use super::users_page::admin_users_page;
use crate::action::ActionError;
use crate::admin::actions::{
    AdminUserProblem, TargetUser, delete_user, disable_user, enable_user, resend_verification,
    sign_out_user,
};
use crate::flash::{Flash, set_flash};
use crate::web::{env, redisplay, session};

struct AdminActionFailed {
    problem: AdminUserProblem,
}

#[route(POST "/admin/users/disable")]
pub async fn disable_user_submit(cx: &Cx, Form(input): Form<TargetUser>) -> Result<SeeOther> {
    let caller = require_admin(cx).await?;
    let outcome = disable_user(&env(cx), &caller, &input).await?;
    finish(cx, &input, outcome, Flash::UserDisabled).await
}

#[route(POST "/admin/users/enable")]
pub async fn enable_user_submit(cx: &Cx, Form(input): Form<TargetUser>) -> Result<SeeOther> {
    let caller = require_admin(cx).await?;
    let outcome = enable_user(&env(cx), &caller, &input).await?;
    finish(cx, &input, outcome, Flash::UserEnabled).await
}

#[route(POST "/admin/users/sign-out")]
pub async fn sign_out_user_submit(cx: &Cx, Form(input): Form<TargetUser>) -> Result<SeeOther> {
    let caller = require_admin(cx).await?;
    let outcome = sign_out_user(&env(cx), &caller, &input).await?;
    finish(cx, &input, outcome, Flash::UserSignedOut).await
}

#[route(POST "/admin/users/resend-verification")]
pub async fn resend_verification_submit(
    cx: &Cx,
    Form(input): Form<TargetUser>,
) -> Result<SeeOther> {
    let caller = require_admin(cx).await?;
    let outcome = resend_verification(&env(cx), &caller, &input).await?;
    finish(cx, &input, outcome, Flash::VerificationResent).await
}

#[route(POST "/admin/users/delete")]
pub async fn delete_user_submit(cx: &Cx, Form(input): Form<TargetUser>) -> Result<SeeOther> {
    let caller = require_admin(cx).await?;
    match delete_user(&env(cx), &caller, &input).await? {
        Ok(()) => {
            set_flash(&session(cx)?, Flash::UserDeleted).await?;
            Ok(see_other(href!(admin_users_page).resolve(cx)))
        }
        Err(error) => fail(cx, &input, error),
    }
}

async fn finish(
    cx: &Cx,
    input: &TargetUser,
    outcome: std::result::Result<(), ActionError<AdminUserProblem>>,
    flash: Flash,
) -> Result<SeeOther> {
    match outcome {
        Ok(()) => {
            set_flash(&session(cx)?, flash).await?;
            Ok(see_other(
                href!(admin_user_page, id: input.id.as_str()).resolve(cx),
            ))
        }
        Err(error) => fail(cx, input, error),
    }
}

fn fail(cx: &Cx, input: &TargetUser, error: ActionError<AdminUserProblem>) -> Result<SeeOther> {
    match error.for_browser()? {
        AdminUserProblem::NotFound => Ok(see_other(href!(admin_users_page).resolve(cx))),
        problem => {
            let path = href!(admin_user_page, id: input.id.as_str()).resolve(cx);
            Err(redisplay(path, AdminActionFailed { problem }))
        }
    }
}

pub(super) fn admin_action_problem(cx: &Cx) -> Option<AdminUserProblem> {
    try_context::<AdminActionFailed>(cx).map(|failed| failed.problem)
}
