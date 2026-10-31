use leptos::prelude::*;

use super::StyleguideSection;

#[component]
pub fn TextSection() -> impl IntoView {
    view! {
        <StyleguideSection title="Text, lists, and links (h2)">
            <p>"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Curabitur id massa quam."</p>
            <p>
                "Vestibulum ut libero risus. Ut ac nulla vitae sapien iaculis euismod eu quis massa. Aliquam "
                <a href="/">"condimentum vitae"</a>
                " dolor vel laoreet. Phasellus mattis molestie commodo. Praesent et nibh odio. Pellentesque eget fermentum lacus. Praesent laoreet velit "
                <a href="/">"non posuere euismod"</a> "."
            </p>
            <ul>
                <li>"Etiam accumsan leo eget"</li>
                <li>"Maecenas tempus volutpat justo"</li>
                <li>"Vivamus ipsum tortor"</li>
                <li>"Aliquam at velit a tortor viverra commodo in non dolor"</li>
            </ul>
        </StyleguideSection>
    }
}
