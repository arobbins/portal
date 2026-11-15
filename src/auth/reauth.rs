use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use topcoat::Result;
use tower_sessions::Session;

use crate::action::{ActionProblem, Outcome, ProblemSpec, fail};
use crate::auth::password::{
    CURRENT_PASSWORD_FIELD, CurrentPasswordProblem, check_current_password,
};
use crate::env::Env;
use crate::user::model::User;

const PASSKEY_REAUTH: &str = "passkey_reauth";
const PASSKEY_REAUTH_WINDOW: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PasskeyReauth {
    user_id: i32,
    session_version: i32,
    confirmed_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasskeyProof {
    user_id: i32,
    grant: PasskeyReauth,
}

#[cfg(test)]
impl PasskeyProof {
    pub fn for_tests(user_id: i32) -> Self {
        let grant = PasskeyReauth {
            user_id,
            session_version: 0,
            confirmed_at: 0,
        };
        Self { user_id, grant }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReauthProblem {
    Missing,
    Password(CurrentPasswordProblem),
}

impl ActionProblem for ReauthProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::Missing => ProblemSpec::new(
                "reauth_required",
                "Enter your current password or confirm with a passkey.",
            )
            .on_field(CURRENT_PASSWORD_FIELD),
            Self::Password(problem) => problem.spec(),
        }
    }
}

pub async fn check_reauth(
    env: &Env<'_>,
    user: &User,
    password: &str,
    passkey: Option<PasskeyProof>,
) -> Outcome<(), ReauthProblem> {
    if passkey.is_some_and(|proof| proof.user_id == user.id) {
        return Ok(Ok(()));
    }
    if password.is_empty() {
        return fail(ReauthProblem::Missing);
    }

    Ok(check_current_password(env, user, password)
        .await?
        .map_err(ReauthProblem::Password))
}

pub async fn grant_passkey_reauth(
    session: &Session,
    user: &User,
    now: OffsetDateTime,
) -> Result<()> {
    let grant = PasskeyReauth {
        user_id: user.id,
        session_version: user.session_version,
        confirmed_at: now.unix_timestamp(),
    };
    session.insert(PASSKEY_REAUTH, grant).await?;
    Ok(())
}

pub async fn take_passkey_proof(
    session: &Session,
    user: &User,
    now: OffsetDateTime,
) -> Result<Option<PasskeyProof>> {
    let grant = session.remove::<PasskeyReauth>(PASSKEY_REAUTH).await?;

    Ok(grant
        .filter(|grant| is_fresh(grant, user, now))
        .map(|grant| PasskeyProof {
            user_id: grant.user_id,
            grant,
        }))
}

pub async fn return_passkey_proof(session: &Session, proof: Option<PasskeyProof>) -> Result<()> {
    if let Some(proof) = proof {
        session.insert(PASSKEY_REAUTH, proof.grant).await?;
    }
    Ok(())
}

pub async fn forget_passkey_proof(session: &Session) -> Result<()> {
    session.remove::<PasskeyReauth>(PASSKEY_REAUTH).await?;
    Ok(())
}

fn is_fresh(grant: &PasskeyReauth, user: &User, now: OffsetDateTime) -> bool {
    let age = now.unix_timestamp() - grant.confirmed_at;
    grant.user_id == user.id
        && grant.session_version == user.session_version
        && (0..=PASSKEY_REAUTH_WINDOW.as_secs() as i64).contains(&age)
}

#[cfg(test)]
mod tests;
