use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

#[component]
pub async fn tabs(cx: &Cx, label: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <nav class="a-tabs" aria-label=(label)>
            <ul class="a-tabs__list">(child)</ul>
        </nav>
    })
}

#[component]
pub async fn tab(
    cx: &Cx,
    href: &str,
    #[default] active: bool,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let class = if active {
        "a-tabs__tab a-tabs__tab--active"
    } else {
        "a-tabs__tab"
    };

    Ok(view! {
        <li class="a-tabs__item">
            <a class=(class) href=(href) aria-current=(active.then_some("page"))>
                (child)
            </a>
        </li>
    })
}
