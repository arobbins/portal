mod create;
mod manage;

pub use create::{CreateToken, CreateTokenProblem, create_api_token};
pub use manage::{RevokeToken, list_api_tokens, revoke_api_token};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionError, Caller, Denied};
    use crate::api::scope::{ApiScope, TokenAudience};
    use crate::api_tokens::lifetime::TokenLifetime;
    use crate::api_tokens::testing::issue_token;
    use crate::auth::password::hash_password;
    use crate::env::App;
    use crate::user::queries::{NewUser, query_insert_user, query_set_password_and_verify_user};
    use sqlx::PgPool;
    use std::sync::Arc;

    #[sqlx::test]
    async fn an_api_token_can_never_create_or_revoke_tokens(db: PgPool) {
        let user_id = query_insert_user(
            &NewUser {
                name: "Guarded",
                email: "guarded@example.test",
            },
            &db,
        )
        .await
        .unwrap()
        .unwrap();
        let hash = hash_password("a-strong-test-password").unwrap();
        let user = Arc::new(
            query_set_password_and_verify_user(user_id, &hash, &db)
                .await
                .unwrap(),
        );
        let existing = issue_token(user_id, &["profile:write"], time::Duration::days(1), &db).await;
        let app = App::for_tests(db.clone());
        let env = app.env(None);
        let caller = Caller::api_token(user, existing.id, ApiScope::ALL.to_vec());

        let created = create_api_token(
            &env,
            &caller,
            &CreateToken {
                name: "Escalation".to_string(),
                access: ApiScope::ProfileWrite,
                lifetime: TokenLifetime::Year,
                audience: TokenAudience::Api,
                current_password: "a-strong-test-password".to_string(),
            },
            None,
        )
        .await
        .unwrap();
        let revoked = revoke_api_token(
            &env,
            &caller,
            &RevokeToken {
                id: existing.public_id,
            },
        )
        .await
        .unwrap();

        assert_eq!(created, Err(ActionError::Denied(Denied::BrowserOnly)));
        assert_eq!(revoked, Err(ActionError::Denied(Denied::BrowserOnly)));
        let caller = Caller::browser(caller.user.clone());
        assert_eq!(
            list_api_tokens(&env, &caller).await.unwrap().unwrap().len(),
            1
        );
        let events = sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM account_events WHERE user_id = $1"#,
            user_id
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(events, 0);
    }

    #[sqlx::test]
    async fn a_passkey_confirmation_can_stand_in_for_the_password(db: PgPool) {
        use crate::auth::reauth::{PasskeyProof, ReauthProblem};

        let user_id = query_insert_user(
            &NewUser {
                name: "Passkey Holder",
                email: "holder@example.test",
            },
            &db,
        )
        .await
        .unwrap()
        .unwrap();
        let hash = hash_password("a-strong-test-password").unwrap();
        let user = Arc::new(
            query_set_password_and_verify_user(user_id, &hash, &db)
                .await
                .unwrap(),
        );
        let app = App::for_tests(db.clone());
        let env = app.env(None);
        let caller = Caller::browser(user);
        let input = CreateToken {
            name: "Deploy".to_string(),
            access: ApiScope::ProfileRead,
            lifetime: TokenLifetime::Year,
            audience: TokenAudience::Api,
            current_password: String::new(),
        };

        let refused = create_api_token(&env, &caller, &input, None).await.unwrap();
        let proof = Some(PasskeyProof::for_tests(user_id));
        let created = create_api_token(&env, &caller, &input, proof)
            .await
            .unwrap();

        assert_eq!(
            refused,
            Err(ActionError::Problem(CreateTokenProblem::Reauth(
                ReauthProblem::Missing
            )))
        );
        assert!(created.is_ok());
    }
}
