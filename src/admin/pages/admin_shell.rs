use topcoat::{
    Result,
    context::Cx,
    runtime::signal,
    view::{Child, View, component, view},
};

use crate::account::pages::account_shell::account_shell;
use crate::account::pages::section_header::{SubPage, section_header};
use crate::account::tabs::AccountArea;
use crate::admin::sections::AdminSection;
use crate::user::model::User;

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
