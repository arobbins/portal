use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::signal,
    view::{Child, View, component, view},
};

use super::feedback_page::admin_feedback_page;
use super::log_page::admin_log_page;
use super::users_page::admin_users_page;
use crate::account::pages::account_shell::account_shell;
use crate::account::pages::section_header::{SubPage, section_header};
use crate::account::tabs::AccountArea;
use crate::user::model::User;

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

#[component]
pub async fn admin_shell(
    cx: &Cx,
    user: &User,
    section: AdminSection,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let display_name = signal(cx, || user.name.clone());
    let pages: Vec<SubPage> = AdminSection::ALL
        .into_iter()
        .map(|item| SubPage {
            label: item.label(),
            href: item.path(cx),
            current: item == section,
        })
        .collect();

    Ok(view! {
        account_shell(
            user: user,
            display_name: &display_name,
            active: AccountArea::Admin(section),
            <div class="p-account__sub-page-content">
                section_header(title: "Admin", pages: &pages)
                <div class="p-admin">(child)</div>
            </div>
        )
    })
}
