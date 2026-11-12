use topcoat::{
    Result,
    context::Cx,
    view::{Child, View, component, view},
};

#[component]
pub async fn menu(cx: &Cx, id: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div id=(id) class="a-menu" popover="">
            <ul class="a-menu__list">(child)</ul>
        </div>
    })
}

#[component]
pub async fn menu_link(cx: &Cx, href: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <li class="a-menu__entry"><a class="a-menu__item" href=(href)>(child)</a></li>
    })
}

#[component]
pub async fn menu_form_button(
    cx: &Cx,
    action: &str,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <li class="a-menu__entry">
            <form class="a-menu__form" method="post" action=(action)>
                <button type="submit" class="a-menu__item">(child)</button>
            </form>
        </li>
    })
}
