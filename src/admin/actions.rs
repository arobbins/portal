use std::convert::Infallible;

use schemars::JsonSchema;
use serde::Deserialize;
use sqlx::PgConnection;
use topcoat::router::StatusCode;

use super::queries::{UserCounts, UserListing, query_count_user_things, query_search_users};
use crate::action::{ActionProblem, ActionResult, Caller, Outcome, ProblemSpec, authorize, fail};
use crate::activity::actions::{ActivityView, activity_of};
use crate::env::Env;
use crate::permission::Permission;
use crate::secret::PublicIdKind;
use crate::user::model::{Role, User};
use crate::user::queries::{query_get_user_by_public_id, query_lock_user_by_public_id};

mod confirm;
mod log;
mod manage;

pub use confirm::{ConfirmAdmin, confirm_admin};
pub use log::{LOG_ENTRIES_SHOWN, list_admin_log};
pub use manage::{delete_user, disable_user, enable_user, resend_verification, sign_out_user};

pub const USERS_PER_PAGE: i64 = 50;
const MAX_SEARCH_LENGTH: usize = 254;

#[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
#[schemars(description = "Find users by email or name.")]
pub struct FindUsers {
    #[serde(default)]
    #[schemars(description = "Part of an email address or name.", length(max = MAX_SEARCH_LENGTH))]
    pub q: String,
    #[serde(default)]
    #[schemars(description = "Which page of results, starting at 0.")]
    pub page: u32,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "The user an admin action applies to.")]
pub struct TargetUser {
    #[schemars(description = "The user's id (usr_…).")]
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserPage {
    pub users: Vec<UserListing>,
    pub page: u32,
    pub has_next: bool,
}

#[derive(Debug)]
pub struct UserOverview {
    pub user: User,
    pub counts: UserCounts,
    pub activity: Vec<ActivityView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminUserProblem {
    NotFound,
    Yourself,
    AnotherAdmin,
    AlreadyHasPassword,
    Disabled,
    TooManyEmails,
}

impl ActionProblem for AdminUserProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::NotFound => ProblemSpec::new("user_not_found", "That user doesn't exist.")
                .with_status(StatusCode::NOT_FOUND),
            Self::Yourself => ProblemSpec::new(
                "own_account",
                "You can't do that to your own account here. Use your account settings instead.",
            ),
            Self::AnotherAdmin => ProblemSpec::new(
                "user_is_admin",
                "Admins can't be changed here. Remove their admin role on the server first.",
            ),
            Self::AlreadyHasPassword => ProblemSpec::new(
                "already_confirmed",
                "This user has already confirmed their email and set a password.",
            ),
            Self::Disabled => ProblemSpec::new(
                "user_disabled",
                "This account is disabled. Enable it before sending it email.",
            ),
            Self::TooManyEmails => ProblemSpec::new(
                "too_many_emails",
                "This address has had several emails recently. Try again in an hour.",
            )
            .with_status(StatusCode::TOO_MANY_REQUESTS),
        }
    }
}

pub async fn list_users(
    env: &Env<'_>,
    caller: &Caller,
    input: &FindUsers,
) -> ActionResult<UserPage, Infallible> {
    authorize!(caller.require(Permission::AdministerUsers));
    let search: String = input.q.trim().chars().take(MAX_SEARCH_LENGTH).collect();
    let offset = i64::from(input.page) * USERS_PER_PAGE;
    let mut users = query_search_users(&search, USERS_PER_PAGE + 1, offset, env.db()).await?;
    let has_next = users.len() as i64 > USERS_PER_PAGE;
    users.truncate(USERS_PER_PAGE as usize);

    Ok(Ok(UserPage {
        users,
        page: input.page,
        has_next,
    }))
}

pub async fn get_user(
    env: &Env<'_>,
    caller: &Caller,
    input: &TargetUser,
) -> ActionResult<UserOverview, AdminUserProblem> {
    authorize!(caller.require(Permission::AdministerUsers));
    let user = match find_target(env, &input.id).await? {
        Ok(user) => user,
        Err(problem) => return fail(problem),
    };
    let counts = query_count_user_things(user.id, env.db()).await?;
    let activity = activity_of(env, user.id).await?;

    Ok(Ok(UserOverview {
        user,
        counts,
        activity,
    }))
}

async fn find_target(env: &Env<'_>, id: &str) -> Outcome<User, AdminUserProblem> {
    if !PublicIdKind::User.is_valid(id) {
        return fail(AdminUserProblem::NotFound);
    }
    match query_get_user_by_public_id(id, env.db()).await? {
        Some(user) => Ok(Ok(user)),
        None => fail(AdminUserProblem::NotFound),
    }
}

async fn changeable_target(
    env: &Env<'_>,
    caller: &Caller,
    id: &str,
) -> Outcome<User, AdminUserProblem> {
    let user = match find_target(env, id).await? {
        Ok(user) => user,
        Err(problem) => return fail(problem),
    };
    Ok(guard_target(caller, user))
}

async fn locked_target(
    caller: &Caller,
    id: &str,
    tx: &mut PgConnection,
) -> Outcome<User, AdminUserProblem> {
    if !PublicIdKind::User.is_valid(id) {
        return fail(AdminUserProblem::NotFound);
    }
    match query_lock_user_by_public_id(id, tx).await? {
        Some(user) => Ok(guard_target(caller, user)),
        None => fail(AdminUserProblem::NotFound),
    }
}

fn guard_target(caller: &Caller, user: User) -> Result<User, AdminUserProblem> {
    if user.id == caller.user.id {
        return Err(AdminUserProblem::Yourself);
    }
    if user.role == Role::Admin {
        return Err(AdminUserProblem::AnotherAdmin);
    }
    Ok(user)
}

#[cfg(test)]
mod tests;
