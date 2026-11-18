use crate::admin::queries::{ListedStatus, UserListing};
use crate::components::tone::Tone;
use crate::time_zone::Zone;
use crate::user::model::{AccountStatus, Role, User};

pub(super) fn status_label(status: ListedStatus) -> (&'static str, Tone) {
    match status {
        ListedStatus::Active => ("Active", Tone::Success),
        ListedStatus::AwaitingPassword => ("Awaiting password", Tone::Warning),
        ListedStatus::Disabled => ("Disabled", Tone::Error),
    }
}

pub(super) fn listed_status(user: &User) -> ListedStatus {
    match user.status() {
        AccountStatus::Active { .. } => ListedStatus::Active,
        AccountStatus::AwaitingPassword => ListedStatus::AwaitingPassword,
        AccountStatus::Disabled { .. } => ListedStatus::Disabled,
    }
}

pub(super) fn role_label(role: Role) -> &'static str {
    match role {
        Role::Admin => "Admin",
        Role::Subscriber => "Subscriber",
    }
}

pub(super) struct UserRow {
    pub id: String,
    pub name: String,
    pub email: String,
    pub status: (&'static str, Tone),
    pub role: &'static str,
    pub joined: String,
    pub last_active: String,
}

impl UserRow {
    pub fn of(user: &UserListing, zone: &Zone) -> Self {
        Self {
            id: user.public_id.clone(),
            name: user.name.clone(),
            email: user.email.clone(),
            status: status_label(user.status),
            role: role_label(user.role),
            joined: zone.date(user.created_at),
            last_active: user
                .last_active_at
                .map_or_else(|| "Not signed in".to_string(), |at| zone.date(at)),
        }
    }
}
