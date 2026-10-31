use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path, SsrMode,
};

use crate::layout::{Footer, Header};
use crate::pages::{AboutPage, HomePage, NotFoundPage, StyleguidePage, WebTransportPage};
use crate::user::pages::{AccountPage, LoginPage, RegisterPage};
use crate::web::use_globals;

const FONTS_URL: &str = "https://fonts.googleapis.com/css2?family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&family=Space+Mono:ital,wght@0,400;0,700;1,400;1,700&display=swap";

pub fn shell(options: LeptosOptions) -> impl IntoView {
    let body_class = format!("l-flex-col {}", use_globals().current_slug);

    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="color-scheme" content="dark"/>
                <title>"Hello world"</title>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin=""/>
                <link href=FONTS_URL rel="stylesheet"/>
                <link href="/assets/dist/app.css" rel="stylesheet"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options islands=true/>
            </head>
            <body class=body_class>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Header/>
            <main class="l-contain l-flex-1">
                <Routes fallback=NotFoundPage>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/about") view=AboutPage/>
                    <Route path=path!("/webtransport") view=WebTransportPage/>
                    <Route path=path!("/styleguide") view=StyleguidePage/>
                    <Route path=path!("/register") view=RegisterPage ssr=SsrMode::Async/>
                    <Route path=path!("/login") view=LoginPage ssr=SsrMode::Async/>
                    <Route path=path!("/account") view=AccountPage ssr=SsrMode::Async/>
                    <Route path=path!("/account/:page") view=AccountPage ssr=SsrMode::Async/>
                </Routes>
            </main>
            <Footer/>
        </Router>
    }
}
