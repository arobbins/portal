use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
#[derive(Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: String,
    pub photo: Option<String>,
    pub subscriber_status: String,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
    pub verified_at: Option<time::OffsetDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUser {
    pub name: String,
    pub email: String,
    pub photo: Option<String>,
}

#[cfg(feature = "ssr")]
impl From<User> for AccountUser {
    fn from(user: User) -> Self {
        Self {
            name: user.name,
            email: user.email,
            photo: user.photo,
        }
    }
}
