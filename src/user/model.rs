#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum Role {
    Subscriber,
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum SubscriberStatus {
    Active,
    Inactive,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password: Option<String>,
    pub role: Role,
    pub photo: Option<String>,
    pub subscriber_status: SubscriberStatus,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
    pub verified_at: Option<time::OffsetDateTime>,
    pub session_version: i32,
    pub webauthn_id: uuid::Uuid,
}

#[derive(Debug)]
pub struct Passkey {
    pub id: i32,
    pub credential_id: Vec<u8>,
    pub passkey: sqlx::types::Json<webauthn_rs::prelude::Passkey>,
    pub name: String,
    pub backup_eligible: bool,
    pub backed_up: bool,
    pub created_at: time::OffsetDateTime,
    pub last_used_at: Option<time::OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus<'a> {
    AwaitingPassword,
    Active { password_hash: &'a str },
}

impl User {
    pub fn status(&self) -> AccountStatus<'_> {
        match (self.password.as_deref(), self.verified_at) {
            (Some(password_hash), Some(_)) => AccountStatus::Active { password_hash },
            _ => AccountStatus::AwaitingPassword,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(password: Option<&str>, verified: bool) -> User {
        let now = time::OffsetDateTime::now_utc();
        User {
            id: 1,
            name: "Tester".to_string(),
            email: "tester@example.test".to_string(),
            password: password.map(str::to_string),
            role: Role::Subscriber,
            photo: None,
            subscriber_status: SubscriberStatus::Inactive,
            created_at: now,
            updated_at: now,
            verified_at: verified.then_some(now),
            session_version: 0,
            webauthn_id: uuid::Uuid::nil(),
        }
    }

    #[test]
    fn only_a_verified_user_with_a_password_is_active() {
        assert_eq!(user(None, false).status(), AccountStatus::AwaitingPassword);
        assert_eq!(
            user(Some("hash"), true).status(),
            AccountStatus::Active {
                password_hash: "hash"
            }
        );
    }

    #[test]
    fn half_finished_accounts_are_never_active() {
        assert_eq!(user(None, true).status(), AccountStatus::AwaitingPassword);
        assert_eq!(
            user(Some("hash"), false).status(),
            AccountStatus::AwaitingPassword
        );
    }
}
