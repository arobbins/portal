use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;

#[component]
pub async fn text_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Text",
            <div class="styleguide__prose">
                <h1>"Heading 1"</h1>
                <h2>"Heading 2"</h2>
                <h3>"Heading 3"</h3>
                <h4>"Heading 4"</h4>
                <p>
                    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur id massa quam."
                </p>
                <p>
                    "Vestibulum ut libero risus. Ut ac nulla vitae sapien iaculis euismod eu quis massa. Aliquam "
                    <a href="/">"condimentum vitae"</a>
                    " dolor vel laoreet. Phasellus mattis molestie commodo. Praesent et nibh odio. Pellentesque eget fermentum lacus. Praesent laoreet velit "
                    <a href="/">"non posuere euismod"</a>
                    "."
                </p>
                <p>
                    "Run "
                    <code>"cargo sqlx prepare"</code>
                    " after changing a query. "
                    <small>"Small text is for hints and timestamps."</small>
                </p>
                <ul>
                    <li>"Etiam accumsan leo eget"</li>
                    <li>"Maecenas tempus volutpat justo"</li>
                    <li>"Aliquam at velit a tortor viverra commodo in non dolor"</li>
                </ul>
                <ol>
                    <li>"Create an account"</li>
                    <li>"Confirm your email"</li>
                    <li>"Set a password"</li>
                </ol>
                <blockquote>
                    "Passkeys can't be phished or reused on other sites, so they're safer than passwords."
                </blockquote>
            </div>
        )
    })
}
