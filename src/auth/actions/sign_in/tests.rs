use std::net::{IpAddr, Ipv4Addr};

use sqlx::PgPool;

use super::*;
use crate::env::App;
use crate::user::testing::{TEST_PASSWORD, active_user};

fn signing_in(email: &str, password: &str) -> SignIn {
    SignIn {
        email: email.to_string(),
        password: password.to_string(),
    }
}

fn from(last: u8) -> Option<IpAddr> {
    Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, last)))
}

#[sqlx::test]
async fn concurrent_wrong_passwords_are_checked_at_most_five_times(db: PgPool) {
    let app = App::for_tests(db.clone());
    let user = active_user("guessed@example.test", &db).await;
    let mut guesses = tokio::task::JoinSet::new();
    for _ in 0..10 {
        let app = app.clone();
        let input = signing_in(&user.email, "not-the-password");
        guesses.spawn(async move {
            sign_in_with_password(&app.env(from(1)), &input)
                .await
                .unwrap()
                .unwrap_err()
        });
    }

    let answers = guesses.join_all().await;

    let wrong = answers
        .iter()
        .filter(|&&answer| answer == SignInProblem::InvalidCredentials)
        .count();
    assert_eq!(wrong, 5, "{answers:?}");
    let right = signing_in(&user.email, TEST_PASSWORD);
    assert_eq!(
        sign_in_with_password(&app.env(from(1)), &right)
            .await
            .unwrap()
            .unwrap_err(),
        SignInProblem::TooManyAttempts
    );
    assert!(
        sign_in_with_password(&app.env(from(2)), &right)
            .await
            .unwrap()
            .is_ok()
    );
}
