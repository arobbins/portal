use sqlx::{PgPool, Postgres, migrate::MigrateDatabase};

use crate::user::auth::hash_password;
use crate::user::queries::{NewUser, query_insert_user, query_update_user_verified_time};

struct SeedUser {
    name: &'static str,
    email: &'static str,
    password: &'static str,
    verified: bool,
}

const SEED_PASSWORD: &str = "correct-horse-battery-staple";

const SEED_USERS: &[SeedUser] = &[
    SeedUser {
        name: "Verified Tester",
        email: "verified@example.test",
        password: SEED_PASSWORD,
        verified: true,
    },
    SeedUser {
        name: "Unverified Tester",
        email: "unverified@example.test",
        password: SEED_PASSWORD,
        verified: false,
    },
];

pub async fn run(database_url: &str) {
    let database_name = database_name(database_url);

    assert!(
        database_name.ends_with("_test"),
        "refusing to seed `{database_name}`: the seed command only runs against databases whose name ends in `_test`"
    );

    if !Postgres::database_exists(database_url).await.unwrap() {
        Postgres::create_database(database_url).await.unwrap();
    }

    let db = PgPool::connect(database_url).await.unwrap();
    sqlx::migrate!().run(&db).await.unwrap();
    query_reset_users(&db).await.unwrap();

    for user in SEED_USERS {
        let password_hash = hash_password(user.password).unwrap();
        let new_user = NewUser {
            name: user.name,
            email: user.email,
            password_hash: &password_hash,
        };
        let user_id = query_insert_user(&new_user, &db).await.unwrap();

        if user.verified {
            query_update_user_verified_time(user_id, &db).await.unwrap();
        }
    }

    tracing::info!("seeded {} users into `{database_name}`", SEED_USERS.len());
}

fn database_name(database_url: &str) -> &str {
    let path = database_url.rsplit('/').next().unwrap_or_default();
    path.split('?').next().unwrap_or_default()
}

async fn query_reset_users(db: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("TRUNCATE users, email_verification_tokens RESTART IDENTITY CASCADE")
        .execute(db)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::database_name;

    #[test]
    fn reads_database_name_from_url() {
        assert_eq!(
            database_name("postgres://user:pass@localhost:5432/app_test"),
            "app_test"
        );
        assert_eq!(
            database_name("postgres://localhost/app_test?sslmode=disable"),
            "app_test"
        );
        assert_eq!(database_name("postgres://localhost/app"), "app");
    }
}
