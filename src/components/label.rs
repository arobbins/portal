use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, ViewExt, component, view},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LabelSize {
    Sm,
    #[default]
    Md,
}

impl LabelSize {
    pub fn class(self) -> &'static str {
        match self {
            Self::Sm => "a-label a-label--sm",
            Self::Md => "a-label a-label--md",
        }
    }
}

#[component]
pub async fn label(
    cx: &Cx,
    #[default] size: LabelSize,
    #[default] for_id: Option<&str>,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let class = size.class();

    if let Some(for_id) = for_id {
        let class = format!("{class} a-label--for-field");
        return Ok(view! { <label class=(class) for=(for_id)>(child)</label> }.boxed());
    }

    Ok(view! { <p class=(class)>(child)</p> }.boxed())
}
