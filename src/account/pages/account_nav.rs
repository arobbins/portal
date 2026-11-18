use crate::routes::API_DOCS_PATH;
use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::account::tabs::{AccountArea, AccountTab};
use crate::admin::pages::admin_shell::AdminSection;
use crate::components::icon::{IconName, icon};

struct NavPage {
    label: &'static str,
    href: String,
    current: bool,
}

struct NavSection {
    label: &'static str,
    icon: IconName,
    href: String,
    active: bool,
    current: bool,
    pages: Vec<NavPage>,
}

impl NavSection {
    fn of(label: &'static str, icon: IconName, pages: Vec<NavPage>) -> Self {
        let href = pages[0].href.clone();
        let active = pages.iter().any(|page| page.current);
        let single = pages.len() == 1;
        Self {
            label,
            icon,
            href,
            active,
            current: single && active,
            pages: if single { Vec::new() } else { pages },
        }
    }
}

fn sections(cx: &Cx, active: AccountArea, is_admin: bool) -> Vec<NavSection> {
    let mut sections: Vec<NavSection> = AccountTab::ALL
        .into_iter()
        .map(|tab| {
            let pages = tab
                .pages()
                .iter()
                .map(|page| NavPage {
                    label: page.heading(),
                    href: page.path(),
                    current: active == AccountArea::Page(*page),
                })
                .collect();
            NavSection::of(tab.heading(), tab_icon(tab), pages)
        })
        .collect();
    if is_admin {
        let pages = AdminSection::ALL
            .into_iter()
            .map(|section| NavPage {
                label: section.label(),
                href: section.path(cx),
                current: active == AccountArea::Admin(section),
            })
            .collect();
        sections.push(NavSection::of("Admin", IconName::UserSquare, pages));
    }
    sections
}

fn tab_icon(tab: AccountTab) -> IconName {
    match tab {
        AccountTab::Profile => IconName::User,
        AccountTab::Security => IconName::Passkey,
        AccountTab::Billing => IconName::CreditCard,
        AccountTab::Settings => IconName::Settings,
        AccountTab::Developer => IconName::Monitor,
    }
}

#[component]
pub async fn account_nav(cx: &Cx, active: AccountArea, is_admin: bool) -> Result<impl View> {
    let sections = sections(cx, active, is_admin);
    let external_links = [
        ("Support", "/support".to_string()),
        ("Documentation", API_DOCS_PATH.to_string()),
        ("System status", "/status".to_string()),
    ];

    Ok(view! {
        <ul class="l-nav__list l-nav__list--flush-left l-flex-col p-account__nav-list">
            for section in &sections {
                <li class="l-nav__list-item l-nav__list-item-group">
                    nav_link(
                        href: section.href.as_str(),
                        active: section.active,
                        current: section.current,
                        leading_icon: Some(section.icon),
                        text: section.label
                    )
                    if !section.pages.is_empty() {
                        <ul class="l-nav__list p-account__nav-links">
                            for page in &section.pages {
                                <li class="l-nav__list-item">
                                    nav_link(
                                        href: page.href.as_str(),
                                        active: page.current,
                                        current: page.current,
                                        text: page.label
                                    )
                                </li>
                            }
                        </ul>
                    }
                </li>
            }
            for (label, href) in &external_links {
                <li class="l-nav__list-item p-account__nav-external">
                    <a href=(href.as_str()) class="l-nav__link l-nav__link-external">
                        (*label)
                        " "
                        icon(name: IconName::ExternalLink)
                    </a>
                </li>
            }
        </ul>
    })
}

#[component]
async fn nav_link(
    cx: &Cx,
    href: &str,
    active: bool,
    current: bool,
    #[default] leading_icon: Option<IconName>,
    text: &str,
) -> Result<impl View> {
    let class = match (active, current) {
        (_, true) => "l-nav__link p-account__nav-link--is-current",
        _ => "l-nav__link",
    };

    Ok(view! {
        <a href=(href) aria-current=(current.then_some("page")) class=(class)>
            if let Some(name) = leading_icon {
                icon(name: name)
            }
            (text)
        </a>
    })
}
