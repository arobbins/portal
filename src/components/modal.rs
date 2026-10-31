use leptos::prelude::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ClosedBy {
    #[default]
    Any,
    CloseRequest,
    None,
}

impl ClosedBy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::CloseRequest => "closerequest",
            Self::None => "none",
        }
    }
}

#[component]
pub fn Modal(
    #[prop(into)] id: String,
    #[prop(into)] heading: String,
    #[prop(optional)] closed_by: ClosedBy,
    children: Children,
) -> impl IntoView {
    let close_target = id.clone();

    view! {
        <dialog id=id class="a-modal" closedby=closed_by.as_str()>
            <h2 class="a-modal__heading">{heading}</h2>
            {children()}
            <button
                type="button"
                command="close"
                commandfor=close_target
                aria-label="Close"
                class="a-modal__close-icon"
            >
                "✕"
            </button>
        </dialog>
    }
}
