use topcoat::{
    Result,
    view::{Child, View, component, view},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(dead_code)]
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
pub async fn modal(
    id: &str,
    heading: &str,
    #[default] closed_by: ClosedBy,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let heading_id = format!("{id}-heading");

    Ok(view! {
        <dialog
            id=(id)
            class="a-modal"
            closedby=(closed_by.as_str())
            aria-labelledby=(heading_id.as_str())
        >
            <h2 id=(heading_id.as_str()) class="a-modal__heading">(heading)</h2>
            (child)
            <button
                type="button"
                command="close"
                commandfor=(id)
                aria-label="Close"
                class="a-modal__close-icon"
            >
                "✕"
            </button>
        </dialog>
    })
}
