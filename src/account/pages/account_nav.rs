use crate::routes::api_docs_url;
use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::account::tabs::AccountTab;
use crate::components::icon::{IconName, icon};

struct NavTab {
    tab: AccountTab,
    href: String,
    links: Vec<(&'static str, String)>,
}

#[component]
pub async fn account_nav(cx: &Cx, active: AccountTab) -> Result<impl View> {
    let tabs: Vec<NavTab> = AccountTab::ALL
        .into_iter()
        .map(|tab| NavTab {
            tab,
            href: tab.href(cx),
            links: tab
                .sections()
                .iter()
                .map(|section| (section.label(), section.href(cx)))
                .collect(),
        })
        .collect();
    let external_links = [
        ("Support", "/support".to_string()),
        ("Documentation", api_docs_url(cx)),
        ("System status", "/status".to_string()),
    ];

    Ok(view! {
        <ul class="l-nav__list l-nav__list--flush-left l-flex-col p-account__nav-list">
            for NavTab { tab, href: tab_href, links } in &tabs {
                <li class="l-nav__list-item l-nav__list-item-group">
                    <a
                        href=(tab_href.as_str())
                        class=(if *tab == active {
                            "l-nav__link l-nav__link--active"
                        } else {
                            "l-nav__link"
                        })
                    >
                        (tab.heading())
                    </a>
                    if !links.is_empty() {
                        <ul class="l-nav__list p-account__nav-links">
                            for (label, href) in links {
                                <li class="l-nav__list-item">
                                    <a href=(href.as_str()) class="l-nav__link">(*label)</a>
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
