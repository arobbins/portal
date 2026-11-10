use sqlx::PgPool;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    router::{
        Router, RouterBuilderDiscoverExt, Slot, StatusCode,
        error::{NotFoundError, not_found},
        layout, page,
    },
    runtime::RuntimeLayer,
    view::{View, error_boundary, view},
};

use crate::config::Config;
use crate::layout::{footer::footer, header::header};
use crate::rate_limit::RateLimits;
use topcoat::metadata::Metadata;

use crate::pages::not_found::not_found_page;
use crate::web::globals;
use std::sync::Arc;
use topcoat::router::TrustedProxies;

pub fn router(
    db: PgPool,
    config: Config,
    rate_limits: Arc<RateLimits>,
    assets: AssetBundle,
) -> Router {
    let trusted_proxy_hops = config.trusted_proxy_hops;
    let mut builder = Router::builder()
        .discover()
        .with(db)
        .with(config)
        .with(rate_limits);

    if let Some(hops) = trusted_proxy_hops {
        builder = builder.trusted_proxies(TrustedProxies::new().nearest(hops));
    }

    builder.assets(assets).layer(RuntimeLayer::new()).build()
}

#[page(* "/{*rest}", with(Metadata::new().title(page_title("Page not found"))))]
async fn unknown_page(cx: &Cx) -> Result<()> {
    Err(not_found().into())
}

pub const SITE_NAME: &str = "Webserver";

pub fn page_title(title: &str) -> String {
    format!("{title} · {SITE_NAME}")
}

#[layout("/", with(Metadata::new().title(SITE_NAME)))]
async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let globals = globals(cx);
    let body_class = format!("l-flex-col {}", globals.current_slug);
    let theme = globals.theme;

    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" data-theme=(theme.as_str())>
            <head>
                topcoat::metadata::tags()
                <meta name="color-scheme" content=(theme.color_scheme())>
                <link href="/assets/dist/app.css" rel="stylesheet">
                <script src="/assets/js/theme.js"></script>
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
