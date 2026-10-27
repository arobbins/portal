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
