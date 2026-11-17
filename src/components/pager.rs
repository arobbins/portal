use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::components::button::{ButtonVariant, button};

#[component]
pub async fn pager(
    cx: &Cx,
    #[default] previous: Option<&str>,
    #[default] next: Option<&str>,
) -> Result<impl View> {
    Ok(view! {
        if previous.is_some() || next.is_some() {
            <nav class="a-pager" aria-label="Pages">
                if let Some(previous) = previous {
                    button(
                        variant: ButtonVariant::Secondary,
                        href: Some(previous),
                        "Previous"
                    )
                }
                if let Some(next) = next {
                    <span class="a-pager__next">
                        button(
                            variant: ButtonVariant::Secondary,
                            href: Some(next),
                            "Next"
                        )
                    </span>
                }
            </nav>
        }
    })
}
