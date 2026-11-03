use sqlx::PgPool;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{
        Router, RouterBuilderDiscoverExt, Slot, StatusCode, error::NotFoundError, layout,
        not_found,
    },
    runtime::RouterBuilderRuntimeExt,
    view::{View, error_boundary, view},
};

use crate::layout::{footer::footer, header::header};
use crate::pages::not_found::not_found_page;
use crate::web::globals;

const FONTS_URL: &str = "https://fonts.googleapis.com/css2?family=Inter:ital,opsz,wght@0,14..32,100..900;1,14..32,100..900&family=Space+Mono:ital,wght@0,400;0,700;1,400;1,700&display=swap";

pub fn router(db: PgPool, assets: AssetBundle) -> Router {
    Router::builder()
        .discover()
        .app_context(db)
        .assets(assets)
        .runtime()
        .build()
}

not_found!("/");

#[layout("/")]
async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let body_class = format!("l-flex-col {}", globals(cx).current_slug);

    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="color-scheme" content="dark">
                <title>"Hello world"</title>
                <link rel="preconnect" href="https://fonts.googleapis.com">
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="">
                <link href=(FONTS_URL) rel="stylesheet">
                <link href="/assets/dist/app.css" rel="stylesheet">
                topcoat::runtime::script()
            </head>
            <body class=(body_class)>
                header()
                <main class="l-contain l-flex-1">
                    error_boundary(
                        fallback: |error| {
                            if error.downcast_ref::<NotFoundError>().is_none() {
                                return Err(error);
                            }

                            Ok(view! {
                                (StatusCode::NOT_FOUND)
                                not_found_page()
                            })
                        },
                        (slot)
                    )
                </main>
                footer()
                topcoat::dev::script()
            </body>
        </html>
    })
}
