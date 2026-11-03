use topcoat::{
    Result,
    view::{View, component, view},
};

use super::account::AccountTab;
use crate::components::icon::{IconName, icon};

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
pub async fn account_nav(active: AccountTab) -> Result<impl View> {
    Ok(view! {
        <ul class="nav__list nav__list--flush-left l-flex-col">
            for section in SECTIONS {
                <li class="nav__list-item nav__list-item-group">
                    <a
                        href=(section.tab.href())
                        class=(if section.tab == active { "nav__link nav__link--active" } else { "nav__link" })
                    >
                        (section.label)
                    </a>
                    if !section.links.is_empty() {
                        <ul class="nav__list">
                            for (label, href) in section.links {
                                <li class="nav__list-item">
                                    <a href=(*href) class="nav__link">(*label)</a>
                                </li>
                            }
                        </ul>
                    }
                </li>
            }
            for (label, href) in EXTERNAL_LINKS {
                <li class="nav__list-item">
                    <a href=(*href) class="nav__link nav__link-external">
                        (*label) " " icon(name: IconName::ExternalLink)
                    </a>
                </li>
            }
        </ul>
    })
}
