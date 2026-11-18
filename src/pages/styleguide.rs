mod avatars;
mod badges;
mod buttons;
mod cards;
mod code;
mod forms;
mod icons;
mod labels;
mod modal;
mod modal_forms;
mod navigation;
mod notices;
mod property_list;
mod special_fields;
mod tables;
mod text;
mod theme_switcher;
mod toaster;
mod toggles;
mod tokens;

use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::page,
    view::{Child, View, component, view},
};

use crate::app::page_title;
use crate::layout::docs_sidebar::{DocsNavGroup, DocsNavLink, docs_sidebar};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};

const GROUPS: &[(&str, &[&str])] = &[
    (
        "Foundations",
        &[
            "Tokens",
            "Text",
            "Labels",
            "Icons and layout",
            "Theme switcher",
        ],
    ),
    ("Actions", &["Buttons", "Toggles", "Modals", "Modal forms"]),
    ("Feedback", &["Notices", "Toaster", "Badges", "Avatars"]),
    ("Forms", &["Forms"]),
    ("Content", &["Cards", "Tables", "Property lists", "Code"]),
    ("Navigation", &["Navigation"]),
];

fn section_id(title: &str) -> String {
    title
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

#[page("/styleguide", with(Metadata::new().title(page_title("Styleguide"))))]
pub async fn styleguide_page(cx: &Cx) -> Result<impl View> {
    let groups: Vec<DocsNavGroup> = GROUPS
        .iter()
        .map(|(name, titles)| DocsNavGroup {
            name: name.to_string(),
            links: titles
                .iter()
                .map(|title| DocsNavLink::new(format!("#{}", section_id(title)), *title))
                .collect(),
        })
        .collect();

    Ok(view! {
        <div class="l-docs p-styleguide">
            docs_sidebar(
                title: "Styleguide",
                subtitle: "Components and patterns",
                nav_label: "Styleguide sections",
                groups: &groups
            )
            <div class="l-docs__main">
                <section
                    class="l-docs__section l-docs__section--single l-docs__section--first"
                    id="overview"
                >
                    <div class="l-docs__prose l-docs__prose--wide">
                        page_heading(
                            text: "Styleguide",
                            variant: PageHeadingVariant::Compact
                        )
                        <p class="a-lead">
                            "Every component in the app, in every variant. Build pages from these before writing page-specific markup."
                        </p>
                    </div>
                </section>
                tokens::tokens_section()
                text::text_section()
                labels::labels_section()
                icons::icons_section()
                theme_switcher::theme_switcher_section()
                buttons::buttons_section()
                toggles::toggles_section()
                modal::modal_section()
                modal_forms::modal_forms_section()
                notices::notices_section()
                toaster::toaster_section()
                badges::badges_section()
                avatars::avatars_section()
                forms::forms_section()
                cards::cards_section()
                tables::tables_section()
                property_list::property_list_section()
                code::code_section()
                navigation::navigation_section()
            </div>
        </div>
    })
}

#[component]
async fn styleguide_section(
    cx: &Cx,
    title: &str,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let id = section_id(title);

    Ok(view! {
        <section
            class="l-docs__section l-docs__section--single p-styleguide__section"
            id=(id)
        >
            <div class="l-docs__prose l-docs__prose--wide">
                <h2>(title)</h2>
                <div class="p-styleguide__examples">(child)</div>
            </div>
        </section>
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_ids_are_slugs_of_their_titles() {
        assert_eq!(section_id("Icons and layout"), "icons-and-layout");
        assert_eq!(section_id("Code"), "code");
    }
}
