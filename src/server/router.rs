use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, HeaderName, HeaderValue, REFERRER_POLICY,
    STRICT_TRANSPORT_SECURITY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::{Router, extract::Request, middleware};
use topcoat::asset::AssetBundle;
use topcoat::router::tower::TowerService;
use tower::Layer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::services::ServeDir;
use tower_http::set_header::{SetResponseHeader, SetResponseHeaderLayer};
use tower_http::trace::TraceLayer;

use crate::env::App;
use crate::health;
use crate::web::{CanonicalHost, attach_remote_addr, inject_globals, redirect_to_canonical_host};

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub fn router(app: App, assets: AssetBundle) -> Router {
    let https = app.config.secure_cookies();
    let canonical_host =
        CanonicalHost::from_base_url(app.config.origin()).expect("Config validated APP_BASE_URL");
    let health = health::router(app.db.clone());
    let pages = crate::app::router(app.clone(), assets);

    let site = Router::new()
        .fallback_service(TowerService::new(pages))
        .layer(middleware::from_fn_with_state(app, inject_globals))
        .nest_service("/assets", static_assets())
        .layer(middleware::from_fn(attach_remote_addr))
        .layer(middleware::from_fn_with_state(
            canonical_host,
            redirect_to_canonical_host,
        ));

    with_tracing(with_security_headers(site, https).merge(health))
}

fn static_assets() -> SetResponseHeader<ServeDir, HeaderValue> {
    SetResponseHeaderLayer::if_not_present(CACHE_CONTROL, HeaderValue::from_static("no-cache"))
        .layer(ServeDir::new("assets"))
}

fn with_security_headers(router: Router, https: bool) -> Router {
    let router = router
        .layer(security_header(X_FRAME_OPTIONS, "DENY"))
        .layer(security_header(
            CONTENT_SECURITY_POLICY,
            "frame-ancestors 'none'",
        ))
        .layer(security_header(X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .layer(security_header(
            REFERRER_POLICY,
            "strict-origin-when-cross-origin",
        ));
    if !https {
        return router;
    }
    router.layer(security_header(
        STRICT_TRANSPORT_SECURITY,
        "max-age=31536000; includeSubDomains",
    ))
}

fn with_tracing(router: Router) -> Router {
    router
        .layer(TraceLayer::new_for_http().make_span_with(request_span))
        .layer(PropagateRequestIdLayer::new(REQUEST_ID))
        .layer(SetRequestIdLayer::new(REQUEST_ID, MakeRequestUuid))
}

fn request_span(request: &Request) -> tracing::Span {
    let request_id = request
        .headers()
        .get(REQUEST_ID)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("-");
    tracing::info_span!(
        "request",
        id = %request_id,
        method = %request.method(),
        path = %request.uri().path(),
    )
}

fn security_header(name: HeaderName, value: &'static str) -> SetResponseHeaderLayer<HeaderValue> {
    SetResponseHeaderLayer::if_not_present(name, HeaderValue::from_static(value))
}
