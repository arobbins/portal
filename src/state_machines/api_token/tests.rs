use std::sync::Arc;

use axum::http::StatusCode;
use sqlx::PgPool;
use time::Duration;
use topcoat::asset::AssetBundle;

use super::*;
use crate::action::Caller;
use crate::admin::actions::{TargetUser, delete_user, disable_user, enable_user};
use crate::api::scope::{ApiScope, TokenAudience};
use crate::api_tokens::actions::{CreateToken, RevokeToken, create_api_token, revoke_api_token};
use crate::api_tokens::lifecycle::{delete_long_expired, find_grant};
use crate::api_tokens::lifetime::TokenLifetime;
use crate::api_tokens::token::generate_api_token;
use crate::auth::actions::{ChangePassword, NewPassword, change_password, set_password_from_link};
use crate::auth::links::TokenPurpose;
use crate::env::{App, Clock, Env};
use crate::secret::{hash_secret, random_base62};
use crate::state_machines::testing::{Browser, assets};
use crate::state_machines::{CellResult, failures, write_results};
use crate::user::model::User;
use crate::user::queries::query_get_user_by_id;
use crate::user::testing::{TEST_PASSWORD, active_user};

const ME: &str = "/api/v1/me";
const NEW_PASSWORD: &str = "another-strong-password";

struct Holder {
    app: App,
    admin: Caller,
    user: Arc<User>,
    browser: Browser,
    raw: String,
    issued: bool,
    ip: [u8; 4],
}

#[derive(Debug, Default)]
struct Snapshot {
    created: i64,
    revoked: i64,
    last_used_at: Option<time::OffsetDateTime>,
}

#[sqlx::test]
async fn every_api_token_state_and_event_behaves_as_declared(db: PgPool) {
    let app = App::for_tests(db.clone());
    let assets = assets();
    let admin = admin(&db).await;
    let mut cells = Vec::new();
    for (row, &from) in API_TOKEN.states.iter().enumerate() {
        for (column, &event) in API_TOKEN.events.iter().enumerate() {
            let expected = expect(from, event);
            let cell = match expected {
                Expect::Unreachable(_) => (None, false, Vec::new()),
                _ => run_cell(&app, &assets, &admin, [row, column], from, event).await,
            };
            cells.push(CellResult {
                from,
                event,
                expected,
                actual: cell.0,
                accepted: cell.1,
                broken: cell.2,
            });
        }
    }

    write_results(&API_TOKEN, &cells);
    let failed = failures(&cells);
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

async fn run_cell(
    app: &App,
    assets: &AssetBundle,
    admin: &Caller,
    [row, column]: [usize; 2],
    from: TokenState,
    event: TokenEvent,
) -> (Option<TokenState>, bool, Vec<&'static str>) {
    let email = format!("token-{row}-{column}@example.test");
    let ip = [198, 51, 103, (row * 16 + column) as u8];
    let mut holder = Holder::arrange(app, assets, admin, &email, ip, from).await;
    let before = holder.snapshot().await;
    let was_active = holder.observe().await.0 == S::Active;
    let accepted = holder.apply(event).await;
    let (state, rejected_uniformly) = holder.observe().await;
    let after = holder.snapshot().await;
    let mut broken = Vec::new();
    if holder.stored_raw().await {
        broken.push(ONLY_THE_HASH_IS_STORED);
    }
    if state != S::Active && !rejected_uniformly {
        broken.push(REJECTIONS_LOOK_THE_SAME);
    }
    if state == S::Active && holder.works_for(TokenAudience::Mcp).await {
        broken.push(OWN_AUDIENCE_ONLY);
    }
    let created = i64::from(event == E::Create && accepted);
    let revoked = i64::from(event == E::Revoke && accepted);
    let counted =
        after.created - before.created != created || after.revoked - before.revoked != revoked;
    if event != E::AccountDeleted && counted {
        broken.push(CHANGES_ARE_RECORDED);
    }
    if event == E::Use {
        let stamped = after.last_used_at.is_some();
        let unchanged = after.last_used_at == before.last_used_at;
        if (was_active && !stamped) || (!was_active && !unchanged) {
            broken.push(USE_IS_STAMPED);
        }
    }
    (Some(state), accepted, broken)
}

impl Holder {
    async fn arrange(
        app: &App,
        assets: &AssetBundle,
        admin: &Caller,
        email: &str,
        ip: [u8; 4],
        from: TokenState,
    ) -> Self {
        let user = active_user(email, &app.db).await;
        let mut holder = Self {
            app: app.clone(),
            admin: admin.clone(),
            user,
            browser: Browser::new(app, assets, ip),
            raw: generate_api_token().raw.into_inner(),
            issued: false,
            ip,
        };
        if from != S::New {
            assert!(holder.create().await, "creating a token for {email}");
        }
        match from {
            S::Suspended => assert!(holder.apply(E::AdminDisables).await),
            S::Expired => assert!(holder.apply(E::Expires).await),
            S::Deleted => assert!(holder.apply(E::Revoke).await),
            S::New | S::Active => {}
        }
        assert_eq!(holder.observe().await.0, from, "arranging {email}");
        holder
    }

    async fn apply(&mut self, event: TokenEvent) -> bool {
        match event {
            E::Create => self.create().await,
            E::Use => self.call_api().await.0 == StatusCode::OK,
            E::Revoke => self.revoke().await,
            E::ChangePassword => self.change_password().await,
            E::PasswordReset => self.reset_password().await,
            E::AdminDisables => {
                let target = self.target();
                disable_user(&self.env(), &self.admin, &target)
                    .await
                    .unwrap()
                    .is_ok()
            }
            E::AdminEnables => {
                let target = self.target();
                enable_user(&self.env(), &self.admin, &target)
                    .await
                    .unwrap()
                    .is_ok()
            }
            E::Expires => self.expire().await,
            E::CleanupAWeekLater => {
                let later = App {
                    clock: Clock::Fixed(time::OffsetDateTime::now_utc() + Duration::days(8)),
                    ..self.app.clone()
                };
                delete_long_expired(&later).await.unwrap();
                true
            }
            E::AccountDeleted => {
                let target = self.target();
                delete_user(&self.env(), &self.admin, &target)
                    .await
                    .unwrap()
                    .is_ok()
            }
        }
    }

    async fn observe(&mut self) -> (TokenState, bool) {
        let (status, body) = self.call_api().await;
        if status == StatusCode::OK {
            return (S::Active, true);
        }
        let uniform = status == StatusCode::UNAUTHORIZED && body.contains("\"invalid_token\"");
        let row = sqlx::query!(
            r#"SELECT t.expires_at <= now() AS "expired!", u.disabled_at IS NOT NULL AS "disabled!"
            FROM api_tokens t JOIN users u ON u.id = t.user_id WHERE t.token_hash = $1"#,
            hash_secret(&self.raw),
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap();
        let state = match row {
            None if self.issued => S::Deleted,
            None => S::New,
            Some(row) if row.expired => S::Expired,
            Some(row) if row.disabled => S::Suspended,
            Some(_) => panic!(
                "a live token for {} was refused: {status} {body}",
                self.user.email
            ),
        };
        (state, uniform)
    }

    async fn call_api(&mut self) -> (StatusCode, String) {
        let raw = self.raw.clone();
        let response = self.browser.get_with_token(ME, &raw).await;
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    async fn create(&mut self) -> bool {
        let input = CreateToken {
            name: format!("Token {}", random_base62(6)),
            access: ApiScope::ProfileRead,
            lifetime: TokenLifetime::Month,
            audience: TokenAudience::Api,
            current_password: TEST_PASSWORD.to_string(),
        };
        let caller = self.caller().await;
        let Ok(raw) = create_api_token(&self.env(), &caller, &input, None)
            .await
            .unwrap()
        else {
            return false;
        };
        if !self.issued {
            self.raw = raw.into_inner();
            self.issued = true;
        }
        true
    }

    async fn revoke(&self) -> bool {
        let Some(id) = self.public_id().await else {
            return false;
        };
        let caller = self.caller().await;
        revoke_api_token(&self.env(), &caller, &RevokeToken { id })
            .await
            .unwrap()
            .is_ok()
    }

    async fn change_password(&self) -> bool {
        let input = ChangePassword {
            current_password: TEST_PASSWORD.to_string(),
            password: NEW_PASSWORD.to_string(),
            confirm_password: NEW_PASSWORD.to_string(),
        };
        let caller = self.caller().await;
        change_password(&self.env(), &caller, &input)
            .await
            .unwrap()
            .is_ok()
    }

    async fn reset_password(&self) -> bool {
        let raw = random_base62(32);
        sqlx::query!(
            "INSERT INTO user_tokens (user_id, purpose, token_hash, expires_at) VALUES ($1, $2, $3, now() + interval '1 hour')",
            self.user.id,
            TokenPurpose::PasswordReset.as_str(),
            hash_secret(&raw),
        )
        .execute(&self.app.db)
        .await
        .unwrap();
        let input = NewPassword {
            password: NEW_PASSWORD.to_string(),
            confirm_password: NEW_PASSWORD.to_string(),
            code: String::new(),
        };
        set_password_from_link(&self.env(), TokenPurpose::PasswordReset, &raw, &input)
            .await
            .unwrap()
            .is_ok()
    }

    async fn expire(&self) -> bool {
        let result = sqlx::query!(
            "UPDATE api_tokens SET created_at = now() - interval '2 days', expires_at = now() - interval '1 minute' WHERE token_hash = $1",
            hash_secret(&self.raw),
        )
        .execute(&self.app.db)
        .await
        .unwrap();
        result.rows_affected() == 1
    }

    async fn works_for(&self, audience: TokenAudience) -> bool {
        find_grant(&self.raw, audience, self.app.clock.now(), &self.app.db)
            .await
            .unwrap()
            .is_some()
    }

    async fn stored_raw(&self) -> bool {
        sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM api_tokens WHERE token_hash = $1 OR name = $1 OR last_four = $1) AS "found!""#,
            self.raw,
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap()
    }

    async fn public_id(&self) -> Option<String> {
        sqlx::query_scalar!(
            "SELECT public_id FROM api_tokens WHERE token_hash = $1",
            hash_secret(&self.raw)
        )
        .fetch_optional(&self.app.db)
        .await
        .unwrap()
    }

    async fn snapshot(&self) -> Snapshot {
        sqlx::query_as!(
            Snapshot,
            r#"SELECT
                (SELECT count(*) FROM account_events WHERE user_id = $1 AND kind = 'api_token_created') AS "created!",
                (SELECT count(*) FROM account_events WHERE user_id = $1 AND kind = 'api_token_revoked') AS "revoked!",
                (SELECT last_used_at FROM api_tokens WHERE token_hash = $2) AS last_used_at"#,
            self.user.id,
            hash_secret(&self.raw),
        )
        .fetch_one(&self.app.db)
        .await
        .unwrap()
    }

    async fn caller(&self) -> Caller {
        let user = query_get_user_by_id(self.user.id, &self.app.db)
            .await
            .unwrap()
            .unwrap();
        Caller::browser(Arc::new(user))
    }

    fn env(&self) -> Env<'_> {
        self.app.env(Some(std::net::IpAddr::from(self.ip)))
    }

    fn target(&self) -> TargetUser {
        TargetUser {
            id: self.user.public_id.clone(),
        }
    }
}

async fn admin(db: &PgPool) -> Caller {
    let user = active_user("token-admin@example.test", db).await;
    sqlx::query!(
        "UPDATE users SET role = 'admin', admin_granted_at = now() WHERE id = $1",
        user.id
    )
    .execute(db)
    .await
    .unwrap();
    let user = query_get_user_by_id(user.id, db).await.unwrap().unwrap();
    Caller::browser(Arc::new(user))
}
