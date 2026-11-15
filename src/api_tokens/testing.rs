use sqlx::PgPool;

use super::queries::{NewApiTokenRow, query_delete_api_token, query_insert_api_token};
use super::token::generate_api_token;
use crate::api::scope::TokenAudience;
use crate::secret::PublicIdKind;

pub struct IssuedToken {
    pub raw: String,
    pub id: i32,
    pub public_id: String,
}

pub async fn issue_token(
    user_id: i32,
    scopes: &[&str],
    expires_in: time::Duration,
    db: &PgPool,
) -> IssuedToken {
    let token = generate_api_token();
    let public_id = PublicIdKind::ApiToken.generate();
    let scopes: Vec<String> = scopes.iter().map(|scope| scope.to_string()).collect();
    let row = NewApiTokenRow {
        user_id,
        public_id: &public_id,
        name: "Test token",
        token_hash: &token.hash,
        last_four: &token.last_four,
        scopes: &scopes,
        audience: TokenAudience::Api.as_str(),
        expires_at: time::OffsetDateTime::now_utc() + expires_in,
        max_per_user: 50,
    };
    let id = query_insert_api_token(&row, db).await.unwrap().unwrap();

    IssuedToken {
        raw: token.raw,
        id,
        public_id,
    }
}

pub async fn delete_token(public_id: &str, user_id: i32, db: &PgPool) -> Option<String> {
    query_delete_api_token(public_id, user_id, db)
        .await
        .unwrap()
}
