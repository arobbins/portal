use leptos::prelude::*;

use super::account::AccountTab;
use crate::components::{Icon, IconName};

struct NavSection {
    tab: AccountTab,
    label: &'static str,
    links: &'static [(&'static str, &'static str)],
}

const SECTIONS: &[NavSection] = &[
    NavSection {
        tab: AccountTab::Profile,
        label: "Profile",
        links: &[("Reset password", "/account/profile#password")],
    },
    NavSection {
        tab: AccountTab::Security,
        label: "Security",
        links: &[
            ("Two-factor authentication", "/account/security#2fa"),
            ("Single sign-on", "/account/security#sso"),
        ],
    },
    NavSection {
        tab: AccountTab::Billing,
        label: "Billing",
        links: &[
            ("Subscriptions", "/account/billing#subscriptions"),
            ("Invoices", "/account/billing#invoices"),
            ("Payment methods", "/account/billing#payment-methods"),
        ],
    },
    NavSection {
        tab: AccountTab::Settings,
        label: "Settings",
        links: &[("Notifications", "/account/settings#notifications")],
    },
    NavSection {
        tab: AccountTab::Developer,
        label: "Developer",
        links: &[],
    },
];

const EXTERNAL_LINKS: &[(&str, &str)] = &[
    ("Support", "/support"),
    ("Documentation", "/docs"),
    ("System status", "/status"),
];

#[component]
pub fn AccountNav(active: AccountTab) -> impl IntoView {
    let sections = SECTIONS
        .iter()
        .map(|section| {
            let class = if section.tab == active {
                "nav__link nav__link--active"
            } else {
                "nav__link"
            };

            let links = (!section.links.is_empty()).then(|| {
                view! {
                    <ul class="nav__list">
                        {section
                            .links
                            .iter()
                            .map(|(label, href)| {
                                view! {
                                    <li class="nav__list-item">
                                        <a href=*href class="nav__link">{*label}</a>
                                    </li>
                                }
                            })
                            .collect_view()}
                    </ul>
                }
            });

            view! {
                <li class="nav__list-item nav__list-item-group">
                    <a href=section.tab.href() class=class>{section.label}</a>
                    {links}
                </li>
            }
        })
        .collect_view();

    let external = EXTERNAL_LINKS
        .iter()
        .map(|(label, href)| {
            view! {
                <li class="nav__list-item">
                    <a href=*href class="nav__link nav__link-external">
                        {*label} " " <Icon name=IconName::ExternalLink/>
                    </a>
                </li>
            }
        })
        .collect_view();

    view! {
        <ul class="nav__list nav__list--flush-left l-flex-col">
            {sections}
            {external}
        </ul>
    }
}
