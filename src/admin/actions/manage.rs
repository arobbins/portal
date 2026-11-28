use super::{AdminUserProblem, TargetUser, locked_target};
use crate::action::{ActionResult, Caller, authorize, fail};
use crate::admin::log::{AdminLogAction, log_admin_action};
use crate::auth::emails::account_removed_email;
use crate::auth::links::{delete_every_link, queue_verification_link};
use crate::devices::lifecycle::end_every_device;
use crate::env::Env;
use crate::jobs::enqueue;
use crate::mail::SendEmail;
use crate::oauth::lifecycle::disconnect_all_for_user;
use crate::permission::Permission;
use crate::user::model::AccountStatus;
use crate::user::queries::{
    query_bump_session_version, query_delete_user, query_disable_user, query_enable_user,
};

pub async fn disable_user(
    env: &Env<'_>,
    caller: &Caller,
    input: &TargetUser,
) -> ActionResult<(), AdminUserProblem> {
    authorize!(caller.require(Permission::AdministerUsers));
    let mut tx = env.db().begin().await?;
    let user = match locked_target(caller, &input.id, &mut tx).await? {
        Ok(user) => user,
        Err(problem) => return fail(problem),
    };
    if matches!(user.status(), AccountStatus::Disabled { .. }) {
        return Ok(Ok(()));
    }

    query_disable_user(user.id, env.now(), &mut *tx).await?;
    end_every_device(user.id, &mut *tx).await?;
    delete_every_link(user.id, &mut tx).await?;
    let action = AdminLogAction::AccountDisabled;
    log_admin_action(Some(&caller.user), action, &user, env.ip, &mut tx).await?;
    tx.commit().await?;

    Ok(Ok(()))
}

pub async fn enable_user(
    env: &Env<'_>,
    caller: &Caller,
    input: &TargetUser,
) -> ActionResult<(), AdminUserProblem> {
    authorize!(caller.require(Permission::AdministerUsers));
    let mut tx = env.db().begin().await?;
    let user = match locked_target(caller, &input.id, &mut tx).await? {
        Ok(user) => user,
        Err(problem) => return fail(problem),
    };
    if !matches!(user.status(), AccountStatus::Disabled { .. }) {
        return Ok(Ok(()));
    }

    query_enable_user(user.id, &mut *tx).await?;
    let action = AdminLogAction::AccountEnabled;
    log_admin_action(Some(&caller.user), action, &user, env.ip, &mut tx).await?;
    tx.commit().await?;

    Ok(Ok(()))
}

pub async fn sign_out_user(
    env: &Env<'_>,
    caller: &Caller,
    input: &TargetUser,
) -> ActionResult<(), AdminUserProblem> {
    authorize!(caller.require(Permission::AdministerUsers));
    let mut tx = env.db().begin().await?;
    let user = match locked_target(caller, &input.id, &mut tx).await? {
        Ok(user) => user,
        Err(problem) => return fail(problem),
    };

    query_bump_session_version(user.id, &mut *tx).await?;
    end_every_device(user.id, &mut *tx).await?;
    disconnect_all_for_user(user.id, &mut *tx).await?;
    let action = AdminLogAction::SignedOutEverywhere;
    log_admin_action(Some(&caller.user), action, &user, env.ip, &mut tx).await?;
    tx.commit().await?;

    Ok(Ok(()))
}

pub async fn resend_verification(
    env: &Env<'_>,
    caller: &Caller,
    input: &TargetUser,
) -> ActionResult<(), AdminUserProblem> {
    authorize!(caller.require(Permission::AdministerUsers));
    let mut tx = env.db().begin().await?;
    let user = match locked_target(caller, &input.id, &mut tx).await? {
        Ok(user) => user,
        Err(problem) => return fail(problem),
    };
    match user.status() {
        AccountStatus::AwaitingPassword => {}
        AccountStatus::Disabled { .. } => return fail(AdminUserProblem::Disabled),
        AccountStatus::Active { .. } => return fail(AdminUserProblem::AlreadyHasPassword),
    }
    if !env.limits().allow_email_to(&user.email).await? {
        return fail(AdminUserProblem::TooManyEmails);
    }

    queue_verification_link(env.app, user.id, &user.email, &mut tx).await?;
    let action = AdminLogAction::VerificationResent;
    log_admin_action(Some(&caller.user), action, &user, env.ip, &mut tx).await?;
    tx.commit().await?;

    Ok(Ok(()))
}

pub async fn delete_user(
    env: &Env<'_>,
    caller: &Caller,
    input: &TargetUser,
) -> ActionResult<(), AdminUserProblem> {
    authorize!(caller.require(Permission::AdministerUsers));
    let mut tx = env.db().begin().await?;
    crate::billing::lock_before_deleting(&input.id, &mut tx).await?;
    let user = match locked_target(caller, &input.id, &mut tx).await? {
        Ok(user) => user,
        Err(problem) => return fail(problem),
    };

    if user.status() != AccountStatus::AwaitingPassword {
        enqueue(&SendEmail(account_removed_email(&user.email)), &mut *tx).await?;
    }
    let action = AdminLogAction::AccountDeleted;
    log_admin_action(Some(&caller.user), action, &user, env.ip, &mut tx).await?;
    crate::billing::cancel_before_deleting(user.id, &mut tx).await?;
    query_delete_user(user.id, &mut *tx).await?;
    tx.commit().await?;

    Ok(Ok(()))
}
