use sqlx::PgPool;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{
        Router, RouterBuilderDiscoverExt, Slot, StatusCode, error::NotFoundError, layout, not_found,
    },
    runtime::RouterBuilderRuntimeExt,
    view::{View, error_boundary, view},
};

use crate::layout::{footer::footer, header::header};
use crate::mail::Mailer;
use crate::rate_limit::RateLimits;

use crate::pages::not_found::not_found_page;
use crate::web::globals;
use std::env;
use std::sync::Arc;
use topcoat::router::TrustedProxies;

pub fn router(
    db: PgPool,
    mailer: Mailer,
    rate_limits: Arc<RateLimits>,
    assets: AssetBundle,
) -> Router {
    let mut builder = Router::builder()
        .discover()
        .app_context(db)
        .app_context(mailer)
        .app_context(rate_limits);

    if let Some(hops) = env::var("TRUSTED_PROXY_HOPS")
        .ok()
        .and_then(|hops| hops.parse::<usize>().ok())
    {
        builder = builder.trusted_proxies(TrustedProxies::new().nearest(hops));
    }

    builder.assets(assets).runtime().build()
}

not_found!("/");

#[layout("/")]
async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let globals = globals(cx);
    let body_class = format!("l-flex-col {}", globals.current_slug);
    let theme = globals.theme;

    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" data-theme=(theme.as_str())>
            <head>
                <meta charset="utf-8">
                <meta name="color-scheme" content=(theme.color_scheme())>
                <title>"Hello world"</title>
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

                            Ok(
                                view! {
                                    (StatusCode::NOT_FOUND)
                                    not_found_page()
                                },
                            )
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
