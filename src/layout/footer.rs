use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::web::globals;

#[component]
pub async fn footer(cx: &Cx) -> Result<impl View> {
    let year = globals(cx).year;

    Ok(view! {
        <footer class="l-contain footer">
            <p>
                "© "
                (year)
                " Andrew Robbins / Github / Email"
            </p>
        </footer>
    })
}
