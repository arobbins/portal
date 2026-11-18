use std::sync::Arc;
use std::time::Duration;

use axum::http::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use topcoat::{
    Result,
    context::Cx,
    router::{error::see_other, href, request},
};
use tower_sessions::Session;

use crate::auth::pages::login::login_page;
use crate::devices::known::{known_device_cookie, known_device_token, recognize_device};
use crate::devices::lifecycle::{SignInMethod, end_device, keep_device, load_device, start_device};
use crate::env::{App, Env};
use crate::routes::ACCOUNT_PATH;
use crate::two_factor::challenge::end_second_factor;
use crate::user::model::User;
use crate::web::{config, env, globals, set_response_cookie};

const SESSION_AUTH: &str = "auth";
pub const PASSKEY_PROMPT_GRANT: &str = "passkey_prompt_grant";
pub const PASSKEY_PROMPT_WINDOW: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SessionAuth {
    user_id: i32,
    session_version: i32,
    #[serde(default)]
    device: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct PasskeyPromptGrant {
    user_id: i32,
    granted_at: i64,
}

pub async fn load_signed_in_user(session: &Session, app: &App) -> Result<Option<Arc<User>>> {
    let Some(auth) = session.get::<SessionAuth>(SESSION_AUTH).await? else {
        return Ok(None);
    };
    let user = match auth.device.as_deref() {
        Some(device) => load_device(app, auth.user_id, device).await?,
        None => None,
    };

    match user {
        Some(user) if user.session_version == auth.session_version => Ok(Some(Arc::new(user))),
        _ => {
            session.flush().await?;
            Ok(None)
        }
    }
}

pub async fn current_device(session: &Session) -> Result<Option<String>> {
    let auth = session.get::<SessionAuth>(SESSION_AUTH).await?;
    Ok(auth.and_then(|auth| auth.device))
}

pub fn current_user(cx: &Cx) -> Option<Arc<User>> {
    globals(cx).current_user
}

pub fn viewer_zone(cx: &Cx) -> crate::time_zone::Zone {
    current_user(cx).map_or_else(crate::time_zone::Zone::utc, |user| user.zone())
}

pub fn require_user(cx: &Cx) -> Result<Arc<User>> {
    current_user(cx).ok_or_else(|| see_other(href!(login_page).resolve(cx)).into())
}

pub fn redirect_signed_in_users(cx: &Cx) -> Result<()> {
    if current_user(cx).is_some() {
        return Err(see_other(ACCOUNT_PATH).into());
    }

    Ok(())
}

pub async fn sign_in(cx: &Cx, session: &Session, user: &User, method: SignInMethod) -> Result<()> {
    let headers = request::headers(cx);
    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let env = env(cx);
    start_session(&env, session, user, user_agent, method).await?;
    let known = known_device_token(headers);
    let token = recognize_device(&env, user, known.as_deref(), method, user_agent).await?;
    set_response_cookie(cx, known_device_cookie(&token, config(cx).secure_cookies()));
    Ok(())
}

async fn start_session(
    env: &Env<'_>,
    session: &Session,
    user: &User,
    user_agent: &str,
    method: SignInMethod,
) -> Result<()> {
    let previous = session.get::<SessionAuth>(SESSION_AUTH).await?;
    end_session_device(env.app, session).await?;
    if previous.is_some_and(|auth| auth.user_id != user.id) {
        session.clear().await;
    }
    session.cycle_id().await?;
    revoke_passkey_prompt(session).await?;
    end_second_factor(session).await?;
    let device = start_device(env, user, user_agent, method).await?;
    remember(session, user, device).await
}

pub async fn stay_signed_in(app: &App, session: &Session, user: &User) -> Result<()> {
    let auth = session.get::<SessionAuth>(SESSION_AUTH).await?;
    let device = auth
        .filter(|auth| auth.user_id == user.id)
        .and_then(|auth| auth.device);
    let Some(device) = device else {
        session.flush().await?;
        return Ok(());
    };
    if !keep_device(app, &device, user).await? {
        session.flush().await?;
        return Ok(());
    }
    session.cycle_id().await?;
    remember(session, user, device).await
}

pub async fn sign_out(app: &App, session: &Session) -> Result<()> {
    end_session_device(app, session).await?;
    session.flush().await?;
    Ok(())
}

async fn end_session_device(app: &App, session: &Session) -> Result<()> {
    if let Some(auth) = session.get::<SessionAuth>(SESSION_AUTH).await?
        && let Some(device) = auth.device
    {
        end_device(app, auth.user_id, &device).await?;
    }
    Ok(())
}

async fn remember(session: &Session, user: &User, device: String) -> Result<()> {
    let auth = SessionAuth {
        user_id: user.id,
        session_version: user.session_version,
        device: Some(device),
    };
    session.insert(SESSION_AUTH, auth).await?;
    Ok(())
}

pub async fn grant_passkey_prompt(
    session: &Session,
    user: &User,
    now: OffsetDateTime,
) -> Result<()> {
    session
        .insert(
            PASSKEY_PROMPT_GRANT,
            PasskeyPromptGrant {
                user_id: user.id,
                granted_at: now.unix_timestamp(),
            },
        )
        .await?;

    Ok(())
}

pub async fn passkey_prompt_granted(
    session: &Session,
    user: &User,
    now: OffsetDateTime,
) -> Result<bool> {
    let grant = session
        .get::<PasskeyPromptGrant>(PASSKEY_PROMPT_GRANT)
        .await?;

    Ok(grant.is_some_and(|grant| {
        grant.user_id == user.id && within_prompt_window(grant.granted_at, now.unix_timestamp())
    }))
}

pub async fn revoke_passkey_prompt(session: &Session) -> Result<()> {
    session
        .remove::<PasskeyPromptGrant>(PASSKEY_PROMPT_GRANT)
        .await?;

    Ok(())
}

fn within_prompt_window(granted_at: i64, now: i64) -> bool {
    (0..=PASSKEY_PROMPT_WINDOW.as_secs() as i64).contains(&(now - granted_at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::password::hash_password;
    use crate::user::queries::{
        NewUser, query_get_user_by_id, query_insert_user, query_set_password_and_verify_user,
    };
    use crate::user::testing::active_user;
    use sqlx::PgPool;
    use tower_sessions::MemoryStore;

    #[test]
    fn a_passkey_prompt_grant_lasts_five_minutes() {
        assert!(within_prompt_window(1_000, 1_000));
        assert!(within_prompt_window(1_000, 1_300));
        assert!(!within_prompt_window(1_000, 1_301));
        assert!(!within_prompt_window(1_000, 999));
    }

    #[sqlx::test]
    async fn a_passkey_prompt_grant_belongs_to_one_user_and_one_sign_in(db: PgPool) {
        let user = active_user("tester@example.test", &db).await;
        let other_id = query_insert_user(
            &NewUser {
                name: "Other",
                email: "other@example.test",
            },
            &db,
        )
        .await
        .unwrap()
        .unwrap();
        let other = query_get_user_by_id(other_id, &db).await.unwrap().unwrap();
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        assert!(
            !passkey_prompt_granted(&session, &user, OffsetDateTime::now_utc())
                .await
                .unwrap()
        );

        grant_passkey_prompt(&session, &user, OffsetDateTime::now_utc())
            .await
            .unwrap();
        assert!(
            passkey_prompt_granted(&session, &user, OffsetDateTime::now_utc())
                .await
                .unwrap()
        );
        assert!(
            !passkey_prompt_granted(&session, &other, OffsetDateTime::now_utc())
                .await
                .unwrap()
        );

        let app = App::for_tests(db.clone());
        start_session(&app.env(None), &session, &other, "", SignInMethod::Password)
            .await
            .unwrap();
        assert!(
            !passkey_prompt_granted(&session, &user, OffsetDateTime::now_utc())
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    async fn a_password_change_signs_out_existing_sessions(db: PgPool) {
        let user = active_user("tester@example.test", &db).await;
        let app = App::for_tests(db.clone());
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        start_session(&app.env(None), &session, &user, "", SignInMethod::Password)
            .await
            .unwrap();
        assert!(load_signed_in_user(&session, &app).await.unwrap().is_some());

        let password_hash = hash_password("a-new-test-password").unwrap();
        query_set_password_and_verify_user(user.id, &password_hash, &db)
            .await
            .unwrap();

        assert!(load_signed_in_user(&session, &app).await.unwrap().is_none());
        assert!(load_signed_in_user(&session, &app).await.unwrap().is_none());
    }

    #[sqlx::test]
    async fn a_session_for_a_deleted_user_is_signed_out(db: PgPool) {
        let user = active_user("tester@example.test", &db).await;
        let app = App::for_tests(db.clone());
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        start_session(&app.env(None), &session, &user, "", SignInMethod::Password)
            .await
            .unwrap();
        sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
            .execute(&db)
            .await
            .unwrap();

        assert!(load_signed_in_user(&session, &app).await.unwrap().is_none());
        assert!(session.is_empty().await);
    }

    #[sqlx::test]
    async fn a_session_from_before_devices_is_signed_out(db: PgPool) {
        let user = active_user("legacy@example.test", &db).await;
        let app = App::for_tests(db.clone());
        let session = Session::new(None, Arc::new(MemoryStore::default()), None);
        let legacy = SessionAuth {
            user_id: user.id,
            session_version: user.session_version,
            device: None,
        };
        session.insert(SESSION_AUTH, legacy).await.unwrap();

        assert!(load_signed_in_user(&session, &app).await.unwrap().is_none());
        assert!(session.is_empty().await);
    }
}
