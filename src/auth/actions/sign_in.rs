use schemars::JsonSchema;
use serde::Deserialize;

use crate::action::{ActionProblem, Outcome, ProblemSpec, TooManyAttempts, fail};
use crate::auth::password::password_matches;
use crate::env::Env;
use crate::user::model::{AccountStatus, User};
use crate::user::queries::query_get_user_by_email;
use crate::utils::parse_email;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(description = "Sign in with an email and password.")]
pub struct SignIn {
    #[schemars(description = "The account's email address.")]
    pub email: String,
    #[schemars(description = "The account's password.")]
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignInProblem {
    InvalidCredentials,
    TooManyAttempts,
}

impl ActionProblem for SignInProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::InvalidCredentials => {
                ProblemSpec::new("invalid_credentials", "Invalid email or password.")
            }
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

pub async fn sign_in_with_password(env: &Env<'_>, input: &SignIn) -> Outcome<User, SignInProblem> {
    if !env.limits().allow_password_submission(env.ip).await? {
        return fail(SignInProblem::TooManyAttempts);
    }
    let Some(email) = parse_email(&input.email) else {
        return fail(SignInProblem::InvalidCredentials);
    };
    if env.limits().login_blocked(&email, env.ip).await? {
        return fail(SignInProblem::TooManyAttempts);
    }

    match verified_user(env, &email, &input.password).await? {
        Some(user) => {
            env.limits().clear_failed_logins(&email, env.ip).await?;
            Ok(Ok(user))
        }
        None => {
            env.limits().record_failed_login(&email, env.ip).await?;
            fail(SignInProblem::InvalidCredentials)
        }
    }
}

async fn verified_user(
    env: &Env<'_>,
    email: &str,
    password: &str,
) -> Result<Option<User>, sqlx::Error> {
    let user = query_get_user_by_email(email, env.db()).await?;
    let hash = user.as_ref().and_then(|user| match user.status() {
        AccountStatus::Active { password_hash } => Some(password_hash),
        AccountStatus::AwaitingPassword => None,
    });
    let matches = password_matches(hash, password);

    Ok(user.filter(|_| matches))
}
