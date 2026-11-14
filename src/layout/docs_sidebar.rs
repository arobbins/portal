use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::badge::badge;
use crate::components::label::{LabelSize, label};
use crate::components::tone::Tone;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsNavLink {
    pub href: String,
    pub label: String,
    pub badge: Option<(String, Tone)>,
}

impl DocsNavLink {
    pub fn new(href: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            href: href.into(),
            label: label.into(),
            badge: None,
        }
    }

    pub fn with_badge(mut self, text: impl Into<String>, tone: Tone) -> Self {
        self.badge = Some((text.into(), tone));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocsNavGroup {
    pub name: String,
    pub links: Vec<DocsNavLink>,
}

#[component]
pub async fn docs_sidebar(
    cx: &Cx,
    title: &str,
    subtitle: &str,
    nav_label: &str,
    groups: &[DocsNavGroup],
) -> Result<impl View> {
    Ok(view! {
        <nav class="l-docs__sidebar l-nav" aria-label=(nav_label) data-docs-nav="">
            <p class="l-docs__sidebar-title">(title)</p>
            <p class="l-docs__sidebar-subtitle">(subtitle)</p>
            docs_nav_groups(groups: groups)
        </nav>
        <script src="/assets/js/docs-nav.js" defer=(true)></script>
        <details class="l-docs__menu">
            <summary class="l-docs__menu-toggle">"On this page"</summary>
            <nav
                class="l-docs__menu-body l-nav"
                aria-label=(nav_label)
                data-docs-nav=""
            >
                docs_nav_groups(groups: groups)
            </nav>
        </details>
    })
}

#[component]
async fn docs_nav_groups(cx: &Cx, groups: &[DocsNavGroup]) -> Result<impl View> {
    Ok(view! {
        #[key(group.name.as_str())]
        for group in groups {
            <div class="l-docs__sidebar-group">
                label(size: LabelSize::Sm, (group.name.as_str()))
                <ul class="l-nav__list l-nav__list--sidebar">
                    #[key(link.href.as_str())]
                    for link in &group.links {
                        <li class="l-nav__list-item">
                            <a class="l-nav__link" href=(link.href.as_str())>
                                if let Some((text, tone)) = &link.badge {
                                    <span class="l-docs__nav-badge">
                                        badge(tone: *tone, (text.as_str()))
                                    </span>
                                }
                                (link.label.as_str())
                            </a>
                        </li>
                    }
                </ul>
            </div>
        }
    })
}
