use std::future::Future;
use std::pin::Pin;

use sqlx::PgPool;
use sqlx::error::BoxDynError;
use sqlx::migrate::{Migration, MigrationSource, Migrator};

use super::*;
use crate::action::{ActionError, Denied};
use crate::api::scope::ApiScope;
use crate::env::App;
use crate::user::queries::{NewUser, query_insert_user};
use crate::user::testing::active_user;

const NEWSLETTERS_MIGRATION: i64 = 20261118130000;

fn change(general: Option<bool>, product: Option<bool>) -> UpdateNewsletters {
    UpdateNewsletters {
        general_newsletter: general,
        product_updates: product,
    }
}

fn both(general_newsletter: bool, product_updates: bool) -> NewsletterSettings {
    NewsletterSettings {
        general_newsletter,
        product_updates,
    }
}

async fn events(user_id: i32, db: &PgPool) -> Vec<(String, String, String, Option<i32>)> {
    sqlx::query!(
        "SELECT kind, detail, channel, api_token_id FROM account_events WHERE user_id = $1 AND kind LIKE 'newsletter_%' ORDER BY id",
        user_id
    )
    .fetch_all(db)
    .await
    .unwrap()
    .into_iter()
    .map(|row| {
        (
            row.kind,
            row.detail["newsletter"].as_str().unwrap_or_default().to_string(),
            row.channel,
            row.api_token_id,
        )
    })
    .collect()
}

#[sqlx::test]
async fn a_new_account_starts_on_both_newsletters(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("new@example.test", &db).await;

    let read = get_newsletters(&env, &Caller::browser(user)).await.unwrap();

    assert_eq!(read, Ok(both(true, true)));
}

#[derive(Debug)]
struct Before(Vec<Migration>);

impl MigrationSource<'static> for Before {
    fn resolve(
        self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Migration>, BoxDynError>> + Send + 'static>> {
        Box::pin(async move { Ok(self.0) })
    }
}

#[sqlx::test(migrations = false)]
async fn an_account_from_before_newsletters_starts_on_neither(db: PgPool) {
    let all = sqlx::migrate!();
    let earlier: Vec<Migration> = all
        .iter()
        .filter(|migration| migration.version < NEWSLETTERS_MIGRATION)
        .cloned()
        .collect();
    assert!(
        all.iter()
            .any(|migration| migration.version == NEWSLETTERS_MIGRATION)
    );
    Migrator::new(Before(earlier))
        .await
        .unwrap()
        .run(&db)
        .await
        .unwrap();
    let existing = NewUser {
        name: "Old",
        email: "old@example.test",
    };
    let old_id = query_insert_user(&existing, &db).await.unwrap().unwrap();

    all.run(&db).await.unwrap();

    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let old = crate::user::testing::activate(old_id, &db).await;
    let fresh = active_user("fresh@example.test", &db).await;
    let read = async |user| get_newsletters(&env, &Caller::browser(user)).await.unwrap();
    assert_eq!(read(old).await, Ok(both(false, false)));
    assert_eq!(read(fresh).await, Ok(both(true, true)));
}

#[sqlx::test]
async fn each_newsletter_changes_on_its_own_and_only_real_changes_are_recorded(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("lists@example.test", &db).await;
    let caller = Caller::browser(user.clone());
    let update = async |input| update_newsletters(&env, &caller, &input).await.unwrap();

    assert_eq!(
        update(change(Some(false), None)).await,
        Ok(both(false, true))
    );
    assert_eq!(
        update(change(Some(false), Some(true))).await,
        Ok(both(false, true))
    );
    assert_eq!(update(change(None, None)).await, Ok(both(false, true)));
    assert_eq!(
        update(change(Some(true), Some(false))).await,
        Ok(both(true, false))
    );

    let browser = |kind: &str, newsletter: &str| {
        (
            kind.to_string(),
            newsletter.to_string(),
            "browser".to_string(),
            None,
        )
    };
    assert_eq!(
        events(user.id, &db).await,
        vec![
            browser("newsletter_unsubscribed", "general_newsletter"),
            browser("newsletter_subscribed", "general_newsletter"),
            browser("newsletter_unsubscribed", "product_updates"),
        ]
    );
}

#[sqlx::test]
async fn tokens_need_the_profile_scopes(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let user = active_user("tokens@example.test", &db).await;
    let reader = Caller::api_token(user.clone(), 3, vec![ApiScope::ProfileRead]);
    let writer = Caller::api_token(user.clone(), 9, vec![ApiScope::ProfileWrite]);
    let nothing = Caller::api_token(user.clone(), 4, vec![]);
    let off = change(None, Some(false));

    assert_eq!(
        get_newsletters(&env, &reader).await.unwrap(),
        Ok(both(true, true))
    );
    assert!(matches!(
        get_newsletters(&env, &nothing).await.unwrap(),
        Err(ActionError::Denied(Denied::MissingScope(
            ApiScope::ProfileRead
        )))
    ));
    assert!(matches!(
        update_newsletters(&env, &reader, &off).await.unwrap(),
        Err(ActionError::Denied(Denied::MissingScope(
            ApiScope::ProfileWrite
        )))
    ));
    assert_eq!(events(user.id, &db).await, vec![]);

    assert_eq!(
        update_newsletters(&env, &writer, &off).await.unwrap(),
        Ok(both(true, false))
    );
    assert_eq!(
        events(user.id, &db).await,
        vec![(
            "newsletter_unsubscribed".to_string(),
            "product_updates".to_string(),
            "api_token".to_string(),
            Some(9)
        )]
    );
}

#[test]
fn the_update_schema_takes_either_choice_and_nothing_else() {
    let schema = serde_json::to_value(schemars::schema_for!(UpdateNewsletters)).unwrap();

    assert!(
        schema
            .get("required")
            .is_none_or(|required| required == &serde_json::json!([]))
    );
    assert_eq!(schema["additionalProperties"], serde_json::json!(false));
    assert!(schema["properties"]["general_newsletter"].is_object());
    assert!(schema["properties"]["product_updates"].is_object());
    let partial: UpdateNewsletters = serde_json::from_str(r#"{"product_updates": false}"#).unwrap();
    assert_eq!(partial.general_newsletter, None);
    assert_eq!(partial.product_updates, Some(false));
    assert!(serde_json::from_str::<UpdateNewsletters>(r#"{"weekly": true}"#).is_err());
}
