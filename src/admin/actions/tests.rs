use std::sync::Arc;

use sqlx::PgPool;

use super::*;
use crate::action::{ActionError, Denied};
use crate::admin::queries::ListedStatus;
use crate::api::scope::ApiScope;
use crate::auth::actions::{SignIn, SignInProblem, sign_in_with_password};
use crate::devices::lifecycle::{SignInMethod, load_device, start_device};
use crate::env::App;
use crate::listing::SortDirection;
use crate::user::queries::query_get_user_by_public_id;
use crate::user::queries::{NewUser, query_get_user_by_id, query_insert_user};
use crate::user::testing::{TEST_PASSWORD, active_user};

async fn admin(db: &PgPool) -> Caller {
    admin_with("boss@example.test", db).await
}

async fn admin_with(email: &str, db: &PgPool) -> Caller {
    let user = active_user(email, db).await;
    sqlx::query!(
        "UPDATE users SET role = 'admin', admin_granted_at = now() WHERE id = $1",
        user.id
    )
    .execute(db)
    .await
    .unwrap();
    let user = query_get_user_by_public_id(&user.public_id, db)
        .await
        .unwrap()
        .unwrap();
    Caller::browser(Arc::new(user))
}

fn target(user: &User) -> TargetUser {
    TargetUser {
        id: user.public_id.clone(),
    }
}

async fn event_channels(user_id: i32, db: &PgPool) -> Vec<(String, String)> {
    sqlx::query!(
        "SELECT kind, channel FROM account_events WHERE user_id = $1 AND kind <> 'signed_in' ORDER BY id",
        user_id
    )
    .fetch_all(db)
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.kind, row.channel))
    .collect()
}

#[sqlx::test]
async fn only_admins_in_a_browser_can_use_the_admin_area(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let subscriber = Caller::browser(active_user("plain@example.test", &db).await);
    let admin = admin(&db).await;
    let admin_token = Caller::api_token(admin.user.clone(), 1, ApiScope::ALL.to_vec());
    let find = FindUsers::default();

    assert_eq!(
        list_users(&env, &subscriber, &find).await.unwrap(),
        Err(ActionError::Denied(Denied::NotPermitted(
            Permission::AdministerUsers
        )))
    );
    assert_eq!(
        list_users(&env, &admin_token, &find).await.unwrap(),
        Err(ActionError::Denied(Denied::BrowserOnly))
    );
    let page = list_users(&env, &admin, &find).await.unwrap().unwrap();
    assert_eq!(page.users.len(), 2);
}

#[sqlx::test]
async fn users_are_searched_by_email_or_name_and_paged(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    for index in 0..USERS_PER_PAGE {
        let email = format!("someone-{index}@example.test");
        let new_user = NewUser {
            name: "Someone",
            email: &email,
        };
        query_insert_user(&new_user, &db).await.unwrap();
    }
    let search = |q: &str, page| FindUsers {
        q: q.to_string(),
        page,
        ..FindUsers::default()
    };

    let first = list_users(&env, &admin, &search("", 0))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(first.users.len() as i64, USERS_PER_PAGE);
    assert!(first.has_next);
    let second = list_users(&env, &admin, &search("", 1))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(second.users.len(), 1);
    assert!(!second.has_next);

    let by_email = list_users(&env, &admin, &search("SOMEONE-7@", 0))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_email.users.len(), 1);
    let by_name = list_users(&env, &admin, &search("boss", 0))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_name.users[0].email, "boss@example.test");
}

#[sqlx::test]
async fn users_are_sorted_by_the_chosen_column_in_either_direction(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    for (name, email) in [
        ("Carol", "carol@example.test"),
        ("alice", "zed@example.test"),
        ("Bob", "bob@example.test"),
    ] {
        query_insert_user(&NewUser { name, email }, &db)
            .await
            .unwrap();
    }
    let emails = async |sort, direction| {
        let find = FindUsers {
            sort,
            direction,
            ..FindUsers::default()
        };
        let page = list_users(&env, &admin, &find).await.unwrap().unwrap();
        page.users
            .into_iter()
            .map(|user| user.email)
            .collect::<Vec<_>>()
    };

    assert_eq!(
        emails(UserSort::Joined, SortDirection::Descending).await,
        [
            "bob@example.test",
            "zed@example.test",
            "carol@example.test",
            "boss@example.test"
        ]
    );
    assert_eq!(
        emails(UserSort::Email, SortDirection::Ascending).await,
        [
            "bob@example.test",
            "boss@example.test",
            "carol@example.test",
            "zed@example.test"
        ]
    );
    assert_eq!(
        emails(UserSort::Email, SortDirection::Descending).await,
        [
            "zed@example.test",
            "carol@example.test",
            "boss@example.test",
            "bob@example.test"
        ]
    );
    assert_eq!(
        emails(UserSort::Name, SortDirection::Ascending).await,
        [
            "zed@example.test",
            "bob@example.test",
            "carol@example.test",
            "boss@example.test"
        ]
    );
    assert_eq!(
        emails(UserSort::Status, SortDirection::Ascending).await[0],
        "boss@example.test"
    );
    assert_eq!(FindUsers::default().sort, UserSort::Joined);
    assert_eq!(FindUsers::default().direction, SortDirection::Descending);
}

#[sqlx::test]
async fn disabling_signs_the_user_out_and_blocks_sign_in_until_enabled(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let user = active_user("blocked@example.test", &db).await;
    let device = start_device(&env, &user, "", SignInMethod::Password)
        .await
        .unwrap();
    let sign_in = SignIn {
        email: user.email.clone(),
        password: TEST_PASSWORD.to_string(),
    };

    disable_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .unwrap();
    assert!(load_device(&app, user.id, &device).await.unwrap().is_none());
    assert_eq!(
        sign_in_with_password(&env, &sign_in)
            .await
            .unwrap()
            .map(|_| ()),
        Err(SignInProblem::Disabled)
    );

    enable_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .unwrap();
    assert!(sign_in_with_password(&env, &sign_in).await.unwrap().is_ok());
    assert_eq!(
        event_channels(user.id, &db).await,
        [
            ("account_disabled".to_string(), "admin".to_string()),
            ("account_enabled".to_string(), "admin".to_string())
        ]
    );
}

#[sqlx::test]
async fn admins_cannot_disable_delete_or_sign_out_themselves_or_other_admins(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let other_admin = active_user("deputy@example.test", &db).await;
    sqlx::query!(
        "UPDATE users SET role = 'admin', admin_granted_at = now() WHERE id = $1",
        other_admin.id
    )
    .execute(&db)
    .await
    .unwrap();
    let me = target(&admin.user);
    let them = target(&other_admin);

    let mine = disable_user(&env, &admin, &me).await.unwrap();
    assert_eq!(mine, Err(ActionError::Problem(AdminUserProblem::Yourself)));
    let theirs = delete_user(&env, &admin, &them).await.unwrap();
    assert_eq!(
        theirs,
        Err(ActionError::Problem(AdminUserProblem::AnotherAdmin))
    );
    let out = sign_out_user(&env, &admin, &me).await.unwrap();
    assert_eq!(out, Err(ActionError::Problem(AdminUserProblem::Yourself)));
    let missing = TargetUser {
        id: "usr_nope".to_string(),
    };
    let unknown = get_user(&env, &admin, &missing).await.unwrap().map(|_| ());
    assert_eq!(
        unknown,
        Err(ActionError::Problem(AdminUserProblem::NotFound))
    );
}

#[sqlx::test]
async fn signing_a_user_out_ends_every_device(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let user = active_user("roaming@example.test", &db).await;
    let laptop = start_device(&env, &user, "", SignInMethod::Password)
        .await
        .unwrap();
    let phone = start_device(&env, &user, "", SignInMethod::Passkey)
        .await
        .unwrap();

    sign_out_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .unwrap();

    assert!(load_device(&app, user.id, &laptop).await.unwrap().is_none());
    assert!(load_device(&app, user.id, &phone).await.unwrap().is_none());
    let overview = get_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(overview.counts.devices, 0);
    assert_eq!(overview.activity[0].summary, "Signed out of every device");
    assert_eq!(overview.activity[0].by.as_deref(), Some("By an admin"));
}

#[sqlx::test]
async fn a_confirmation_email_is_resent_only_before_a_password_is_set(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let active = active_user("done@example.test", &db).await;
    let new_user = NewUser {
        name: "Waiting",
        email: "waiting@example.test",
    };
    let waiting_id = query_insert_user(&new_user, &db).await.unwrap().unwrap();
    let waiting = query_get_user_by_id(waiting_id, &db)
        .await
        .unwrap()
        .unwrap();

    let done = resend_verification(&env, &admin, &target(&active))
        .await
        .unwrap();
    assert_eq!(
        done,
        Err(ActionError::Problem(AdminUserProblem::AlreadyHasPassword))
    );
    resend_verification(&env, &admin, &target(&waiting))
        .await
        .unwrap()
        .unwrap();
    let links = sqlx::query_scalar!(
        "SELECT count(*) AS \"count!\" FROM user_tokens WHERE user_id = $1 AND purpose = 'email_verification'",
        waiting.id
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(links, 1);
}

#[sqlx::test]
async fn deleting_a_user_removes_the_account(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let user = active_user("removed@example.test", &db).await;

    delete_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .unwrap();

    let gone = get_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .map(|_| ());
    assert_eq!(gone, Err(ActionError::Problem(AdminUserProblem::NotFound)));
}

#[sqlx::test]
async fn the_listed_status_matches_the_account_status(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let disabled = active_user("off@example.test", &db).await;
    disable_user(&env, &admin, &target(&disabled))
        .await
        .unwrap()
        .unwrap();
    let new_user = NewUser {
        name: "Waiting",
        email: "pending@example.test",
    };
    query_insert_user(&new_user, &db).await.unwrap();
    let status_of = |page: &UserPage, email: &str| {
        page.users
            .iter()
            .find(|user| user.email == email)
            .map(|user| user.status)
    };

    let page = list_users(&env, &admin, &FindUsers::default())
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        status_of(&page, "boss@example.test"),
        Some(ListedStatus::Active)
    );
    assert_eq!(
        status_of(&page, "off@example.test"),
        Some(ListedStatus::Disabled)
    );
    assert_eq!(
        status_of(&page, "pending@example.test"),
        Some(ListedStatus::AwaitingPassword)
    );
    let literal = FindUsers {
        q: "%".to_string(),
        ..FindUsers::default()
    };
    assert!(
        list_users(&env, &admin, &literal)
            .await
            .unwrap()
            .unwrap()
            .users
            .is_empty()
    );
}

#[sqlx::test]
async fn enabling_or_disabling_twice_changes_nothing_the_second_time(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let user = active_user("steady@example.test", &db).await;
    let device = start_device(&env, &user, "", SignInMethod::Password)
        .await
        .unwrap();

    enable_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .unwrap();
    assert!(load_device(&app, user.id, &device).await.unwrap().is_some());

    disable_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .unwrap();
    disable_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        event_channels(user.id, &db).await,
        [("account_disabled".to_string(), "admin".to_string())]
    );
}

#[sqlx::test]
async fn resending_respects_the_per_address_email_limit(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let new_user = NewUser {
        name: "Waiting",
        email: "flooded@example.test",
    };
    let user_id = query_insert_user(&new_user, &db).await.unwrap().unwrap();
    let user = query_get_user_by_id(user_id, &db).await.unwrap().unwrap();
    let mut outcomes = Vec::new();

    for _ in 0..4 {
        outcomes.push(
            resend_verification(&env, &admin, &target(&user))
                .await
                .unwrap(),
        );
    }

    assert_eq!(outcomes[..3], [Ok(()), Ok(()), Ok(())]);
    assert_eq!(
        outcomes[3],
        Err(ActionError::Problem(AdminUserProblem::TooManyEmails))
    );
}

async fn logged(db: &PgPool) -> Vec<(Option<String>, String, String)> {
    sqlx::query!("SELECT admin_email, action, target_email FROM admin_log ORDER BY id")
        .fetch_all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.admin_email, row.action, row.target_email))
        .collect()
}

#[sqlx::test]
async fn every_admin_action_is_logged_and_the_log_outlives_the_account(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let user = active_user("audited@example.test", &db).await;
    let user = target(&user);

    disable_user(&env, &admin, &user).await.unwrap().unwrap();
    enable_user(&env, &admin, &user).await.unwrap().unwrap();
    sign_out_user(&env, &admin, &user).await.unwrap().unwrap();
    delete_user(&env, &admin, &user).await.unwrap().unwrap();

    let by = Some("boss@example.test".to_string());
    let entry = |action: &str| {
        (
            by.clone(),
            action.to_string(),
            "audited@example.test".to_string(),
        )
    };
    assert_eq!(
        logged(&db).await,
        [
            entry("account_disabled"),
            entry("account_enabled"),
            entry("signed_out_everywhere"),
            entry("account_deleted"),
        ]
    );
    let shown = list_admin_log(&env, &admin).await.unwrap().unwrap();
    assert_eq!(shown.len(), 4);
    assert!(shown.iter().all(|entry| !entry.target_exists));
    let subscriber = Caller::browser(active_user("nosy@example.test", &db).await);
    assert_eq!(
        list_admin_log(&env, &subscriber).await.unwrap().map(|_| ()),
        Err(ActionError::Denied(Denied::NotPermitted(
            Permission::AdministerUsers
        )))
    );
}

#[sqlx::test]
async fn an_action_that_changes_nothing_is_not_logged(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let admin = admin(&db).await;
    let user = active_user("calm@example.test", &db).await;

    enable_user(&env, &admin, &target(&user))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(logged(&db).await, []);
}

async fn hold_lock_on(user: &User, db: &PgPool) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut holder = db.begin().await.unwrap();
    sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", user.id)
        .fetch_one(&mut *holder)
        .await
        .unwrap();
    holder
}

async fn release_soon(holder: sqlx::Transaction<'static, sqlx::Postgres>) {
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    holder.commit().await.unwrap();
}

#[sqlx::test]
async fn two_admins_disabling_at_once_disable_the_account_once(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let first = admin_with("first@example.test", &db).await;
    let second = admin_with("second@example.test", &db).await;
    let user = active_user("contested@example.test", &db).await;
    let holder = hold_lock_on(&user, &db).await;
    let input = target(&user);

    let (a, b, ()) = tokio::join!(
        disable_user(&env, &first, &input),
        disable_user(&env, &second, &input),
        release_soon(holder),
    );

    assert_eq!(a.unwrap(), Ok(()));
    assert_eq!(b.unwrap(), Ok(()));
    assert_eq!(
        event_channels(user.id, &db).await,
        [("account_disabled".to_string(), "admin".to_string())]
    );
    assert_eq!(logged(&db).await.len(), 1);
}

#[sqlx::test]
async fn deleting_while_another_admin_disables_leaves_no_half_done_account(db: PgPool) {
    let app = App::for_tests(db.clone());
    let env = app.env(None);
    let first = admin_with("first@example.test", &db).await;
    let second = admin_with("second@example.test", &db).await;
    let user = active_user("contested@example.test", &db).await;
    let holder = hold_lock_on(&user, &db).await;
    let input = target(&user);

    let (deleted, disabled, ()) = tokio::join!(
        delete_user(&env, &first, &input),
        disable_user(&env, &second, &input),
        release_soon(holder),
    );

    assert_eq!(deleted.unwrap(), Ok(()));
    let disabled = disabled.unwrap();
    assert!(
        matches!(
            disabled,
            Ok(()) | Err(ActionError::Problem(AdminUserProblem::NotFound))
        ),
        "{disabled:?}"
    );
    assert!(query_get_user_by_id(user.id, &db).await.unwrap().is_none());
    let actions: Vec<String> = logged(&db).await.into_iter().map(|entry| entry.1).collect();
    assert!(actions.contains(&"account_deleted".to_string()));
    assert_eq!(actions.len(), if disabled.is_ok() { 2 } else { 1 });
}
