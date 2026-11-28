use topcoat::{context::Cx, router::href};

use crate::admin::pages::feedback_page::admin_feedback_page;
use crate::admin::pages::log_page::admin_log_page;
use crate::admin::pages::users_page::admin_users_page;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminSection {
    Users,
    Log,
    Feedback,
}

impl AdminSection {
    pub const ALL: [Self; 3] = [Self::Users, Self::Log, Self::Feedback];

    pub fn label(self) -> &'static str {
        match self {
            Self::Users => "Users",
            Self::Log => "Admin log",
            Self::Feedback => "API feedback",
        }
    }

    pub fn path(self, cx: &Cx) -> String {
        match self {
            Self::Users => href!(admin_users_page).resolve(cx),
            Self::Log => href!(admin_log_page).resolve(cx),
            Self::Feedback => href!(admin_feedback_page).resolve(cx),
        }
    }
}
