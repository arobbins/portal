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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus<'a> {
    AwaitingPassword,
    Active { password_hash: &'a str },
}

impl AccountStatus<'_> {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Active { .. })
    }
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
