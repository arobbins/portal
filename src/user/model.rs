#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub enum Role {
    Subscriber,
    Admin,
}

#[derive(Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password: Option<String>,
    pub role: Role,
    pub verified_at: Option<time::OffsetDateTime>,
    pub session_version: i32,
    pub webauthn_id: uuid::Uuid,
    pub public_id: String,
    pub disabled_at: Option<time::OffsetDateTime>,
    pub admin_granted_at: Option<time::OffsetDateTime>,
    pub time_zone: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus<'a> {
    AwaitingPassword,
    Active { password_hash: &'a str },
    Disabled { password_hash: Option<&'a str> },
}

impl User {
    pub fn zone(&self) -> crate::time_zone::Zone {
        crate::time_zone::Zone::named(self.time_zone.as_deref())
    }

    pub fn status(&self) -> AccountStatus<'_> {
        if self.disabled_at.is_some() {
            let password_hash = self
                .password
                .as_deref()
                .filter(|_| self.verified_at.is_some());
            return AccountStatus::Disabled { password_hash };
        }
        match (self.password.as_deref(), self.verified_at) {
            (Some(password_hash), Some(_)) => AccountStatus::Active { password_hash },
            _ => AccountStatus::AwaitingPassword,
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status(), AccountStatus::Active { .. })
    }
}

#[cfg(test)]
impl User {
    pub fn default_for_tests() -> Self {
        let now = time::OffsetDateTime::now_utc();
        Self {
            id: 1,
            name: "Tester".to_string(),
            email: "tester@example.test".to_string(),
            password: Some("hash".to_string()),
            role: Role::Subscriber,
            verified_at: Some(now),
            session_version: 0,
            webauthn_id: uuid::Uuid::nil(),
            public_id: "usr_00000000000000000000".to_string(),
            disabled_at: None,
            admin_granted_at: None,
            time_zone: None,
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
            verified_at: verified.then_some(now),
            session_version: 0,
            webauthn_id: uuid::Uuid::nil(),
            public_id: "usr_00000000000000000000".to_string(),
            disabled_at: None,
            admin_granted_at: None,
            time_zone: None,
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
