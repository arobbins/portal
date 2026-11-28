use serde_json::json;
use sqlx::PgConnection;

use super::super::codes::{matching_step, normalize_recovery_code};
use super::super::queries::{query_get_two_factor, query_use_recovery_code, query_use_step};
use crate::action::{ActionProblem, Outcome, ProblemSpec, TooManyAttempts, fail};
use crate::env::Env;
use crate::secret::hash_secret;
use crate::user::events::{AccountEventKind, EventSource, record_event};
use crate::user::model::User;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondFactorProblem {
    Wrong,
    TooManyAttempts,
}

impl ActionProblem for SecondFactorProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Wrong => ProblemSpec::new(
                "wrong_code",
                "That code didn't work. Enter the newest code from your app, or one of your recovery codes.",
            )
            .on_field(super::CODE_FIELD),
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

pub async fn needs_second_factor(env: &Env<'_>, user: &User) -> Result<bool, sqlx::Error> {
    Ok(query_get_two_factor(user.id, env.db()).await?.is_some())
}

pub async fn verify_second_factor(
    env: &Env<'_>,
    user: &User,
    code: &str,
) -> Outcome<(), SecondFactorProblem> {
    let limits = env.limits();
    if !limits.allow_password_submission(env.ip).await?
        || !limits.take_second_factor_attempt(&user.email).await?
    {
        return fail(SecondFactorProblem::TooManyAttempts);
    }
    let mut tx = env.db().begin().await?;
    let accepted = second_factor_matches(env, user, code, &mut tx).await?;
    tx.commit().await?;
    if accepted {
        limits.clear_failed_second_factors(&user.email).await?;
        return Ok(Ok(()));
    }
    fail(SecondFactorProblem::Wrong)
}

pub async fn second_factor_matches(
    env: &Env<'_>,
    user: &User,
    code: &str,
    tx: &mut PgConnection,
) -> topcoat::Result<bool> {
    Ok(accepts_app_code(env, user, code, tx).await?
        || accepts_recovery_code(env, user, code, tx).await?)
}

async fn accepts_app_code(
    env: &Env<'_>,
    user: &User,
    code: &str,
    tx: &mut PgConnection,
) -> Result<bool, sqlx::Error> {
    let Some(row) = query_get_two_factor(user.id, &mut *tx).await? else {
        return Ok(false);
    };
    let Some(step) = matching_step(&row.secret, code, env.now().unix_timestamp()) else {
        return Ok(false);
    };
    query_use_step(user.id, step, &mut *tx).await
}

async fn accepts_recovery_code(
    env: &Env<'_>,
    user: &User,
    code: &str,
    tx: &mut PgConnection,
) -> topcoat::Result<bool> {
    let normalized = normalize_recovery_code(code);
    if normalized.len() != 10 {
        return Ok(false);
    }
    let used =
        query_use_recovery_code(user.id, &hash_secret(&normalized), env.now(), &mut *tx).await?;
    if used {
        let source = EventSource::browser(user.id);
        record_event(
            env,
            &source,
            AccountEventKind::RecoveryCodeUsed,
            json!({}),
            &mut *tx,
        )
        .await?;
    }
    Ok(used)
}
