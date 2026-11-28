use crate::routes::API_DOCS_PATH;
use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, Signal},
    view::{View, component, view},
};

use crate::account::tabs::{AccountArea, AccountTab};
use crate::admin::pages::admin_unlock_dialog::ADMIN_UNLOCK_MODAL;
use crate::admin::sections::AdminSection;
use crate::components::icon::{IconName, icon};
use crate::web::config;

struct NavPage {
    label: &'static str,
    href: String,
    current: bool,
}

struct NavSection {
    label: &'static str,
    admin: bool,
    icon: IconName,
    href: String,
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
            admin: false,
            icon,
            href,
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
        let mut admin = NavSection::of("Admin", IconName::UserSquare, pages);
        admin.admin = true;
        sections.push(admin);
    }
    sections
}

fn tab_icon(tab: AccountTab) -> IconName {
    match tab {
        AccountTab::Billing => IconName::CreditCard,
        AccountTab::Settings => IconName::Settings,
        AccountTab::Developer => IconName::Code,
    }
}

#[component]
pub async fn account_nav(
    cx: &Cx,
    active: AccountArea,
    is_admin: bool,
    #[default] admin_unlock: Option<&Signal<String>>,
) -> Result<impl View> {
    let sections = sections(cx, active, is_admin);
    let external_links = external_links(cx);

    Ok(view! {
        <ul class="l-nav__list l-nav__list--flush-left l-flex-col p-account__nav-list">
            for section in &sections {
                <li class="l-nav__list-item l-nav__list-item-group">
                    nav_link(
                        href: section.href.as_str(),
                        current: section.current,
                        leading_icon: Some(section.icon),
                        unlock: admin_unlock.filter(|_| section.admin),
                        text: section.label
                    )
                    if !section.pages.is_empty() {
                        <ul class="l-nav__list p-account__nav-links">
                            for page in &section.pages {
                                <li class="l-nav__list-item">
                                    nav_link(
                                        href: page.href.as_str(),
                                        current: page.current,
                                        unlock: admin_unlock.filter(|_| section.admin),
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
                        icon(name: IconName::ExternalLink)
                    </a>
                </li>
            }
        </ul>
    })
}

fn external_links(cx: &Cx) -> Vec<(&'static str, String)> {
    let config = config(cx);
    [
        ("Support", config.support_url.clone()),
        ("Documentation", Some(API_DOCS_PATH.to_string())),
        ("System status", config.status_url.clone()),
    ]
    .into_iter()
    .filter_map(|(label, href)| Some((label, href?)))
    .collect()
}

#[component]
async fn nav_link(
    cx: &Cx,
    href: &str,
    current: bool,
    #[default] leading_icon: Option<IconName>,
    #[default] unlock: Option<&Signal<String>>,
    text: &str,
) -> Result<impl View> {
    let class = if current {
        "l-nav__link p-account__nav-link--is-current"
    } else {
        "l-nav__link"
    };
    let target = href.to_string();
    let modal_id = ADMIN_UNLOCK_MODAL.to_string();

    Ok(view! {
        if let Some(next) = unlock {
            <a
                href=(href)
                class=(class)
                @click=$(|e: Event| {
                    e.prevent_default();
                    next.set(target.clone());
                    raw!("document.getElementById(String(${modal_id})).showModal()", ());
                })
            >
                if let Some(name) = leading_icon {
                    icon(name: name)
                }
                (text)
            </a>
        } else {
            <a href=(href) aria-current=(current.then_some("page")) class=(class)>
                if let Some(name) = leading_icon {
                    icon(name: name)
                }
                (text)
            </a>
        }
    })
}
