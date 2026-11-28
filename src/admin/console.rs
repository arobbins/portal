use sqlx::PgPool;

use super::log::{AdminLogAction, log_admin_action};
use crate::user::model::Role;
use crate::user::queries::{
    query_get_user_by_email, query_get_user_by_id, query_set_role_by_email,
};
use crate::utils::parse_email;

const USAGE: &str = "usage: webserver admin grant|revoke <email>";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Grant,
    Revoke,
}

impl Change {
    fn parse(word: &str) -> Option<Self> {
        match word {
            "grant" => Some(Self::Grant),
            "revoke" => Some(Self::Revoke),
            _ => None,
        }
    }

    fn role(self) -> Role {
        match self {
            Self::Grant => Role::Admin,
            Self::Revoke => Role::Subscriber,
        }
    }

    fn action(self) -> AdminLogAction {
        match self {
            Self::Grant => AdminLogAction::AdminGranted,
            Self::Revoke => AdminLogAction::AdminRevoked,
        }
    }
}

pub async fn run(database_url: &str, args: &[String]) {
    let (Some(change), Some(email)) = (
        args.first().and_then(|word| Change::parse(word)),
        args.get(1).and_then(|email| parse_email(email)),
    ) else {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    let db = PgPool::connect(database_url)
        .await
        .expect("could not connect to DATABASE_URL");

    match change_role(&db, change, &email).await {
        Ok(RoleOutcome::Changed) => println!("{email} is now {}", role_name(change.role())),
        Ok(RoleOutcome::AlreadySet) => println!("{email} is already {}", role_name(change.role())),
        Ok(RoleOutcome::NoAccount) => {
            eprintln!("no account uses {email}");
            std::process::exit(1);
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleOutcome {
    Changed,
    AlreadySet,
    NoAccount,
}

pub async fn change_role(
    db: &PgPool,
    change: Change,
    email: &str,
) -> Result<RoleOutcome, sqlx::Error> {
    let mut tx = db.begin().await?;
    let Some(user_id) = query_set_role_by_email(email, change.role(), &mut *tx).await? else {
        let exists = query_get_user_by_email(email, &mut *tx).await?.is_some();
        return Ok(if exists {
            RoleOutcome::AlreadySet
        } else {
            RoleOutcome::NoAccount
        });
    };
    let Some(user) = query_get_user_by_id(user_id, &mut *tx).await? else {
        return Ok(RoleOutcome::NoAccount);
    };
    log_admin_action(None, change.action(), &user, None, &mut tx).await?;
    tx.commit().await?;

    Ok(RoleOutcome::Changed)
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Admin => "an admin",
        Role::Subscriber => "a subscriber",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::user::testing::active_user;

    #[sqlx::test]
    async fn granting_and_revoking_changes_the_role_and_records_it(db: PgPool) {
        let user = active_user("promoted@example.test", &db).await;

        let grant = change_role(&db, Change::Grant, &user.email).await.unwrap();
        assert_eq!(grant, RoleOutcome::Changed);
        let role = sqlx::query_scalar!(
            r#"SELECT role AS "role: Role" FROM users WHERE id = $1"#,
            user.id
        )
        .fetch_one(&db)
        .await
        .unwrap();
        assert_eq!(role, Role::Admin);

        let again = change_role(&db, Change::Grant, &user.email).await.unwrap();
        assert_eq!(again, RoleOutcome::AlreadySet);
        let revoke = change_role(&db, Change::Revoke, &user.email).await.unwrap();
        assert_eq!(revoke, RoleOutcome::Changed);
        let nobody = change_role(&db, Change::Grant, "nobody@example.test")
            .await
            .unwrap();
        assert_eq!(nobody, RoleOutcome::NoAccount);
        let events = sqlx::query!(
            "SELECT kind, channel FROM account_events WHERE user_id = $1 ORDER BY id",
            user.id
        )
        .fetch_all(&db)
        .await
        .unwrap();
        let events: Vec<(&str, &str)> = events
            .iter()
            .map(|event| (event.kind.as_str(), event.channel.as_str()))
            .collect();
        assert_eq!(
            events,
            [("admin_granted", "console"), ("admin_revoked", "console")]
        );
        let logged = sqlx::query!("SELECT admin_email, action FROM admin_log ORDER BY id")
            .fetch_all(&db)
            .await
            .unwrap();
        let logged: Vec<(Option<&str>, &str)> = logged
            .iter()
            .map(|row| (row.admin_email.as_deref(), row.action.as_str()))
            .collect();
        assert_eq!(logged, [(None, "admin_granted"), (None, "admin_revoked")]);
    }

    #[test]
    fn only_grant_and_revoke_are_understood() {
        assert_eq!(Change::parse("grant"), Some(Change::Grant));
        assert_eq!(Change::parse("revoke"), Some(Change::Revoke));
        assert_eq!(Change::parse("promote"), None);
    }
}
