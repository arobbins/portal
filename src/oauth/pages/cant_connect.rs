use topcoat::{
    Result,
    context::Cx,
    router::StatusCode,
    view::{View, component, view},
};

use crate::layout::page_heading::page_heading;
use crate::oauth::authorize::CantConnect;

impl CantConnect {
    pub fn heading(self) -> &'static str {
        match self {
            Self::TooManyRequests => "Too many sign-in requests",
            _ => "Can't connect this app",
        }
    }

    pub fn explanation(self) -> &'static str {
        match self {
            Self::UnregisteredRedirect => {
                "This app asked to return to an address it didn't register."
            }
            Self::UnknownClient => {
                "This app isn't registered here, or its registration has expired. Go back to the app and connect again."
            }
            Self::Expired => {
                "This request has expired or was already answered. Go back to the app and connect again."
            }
            Self::TooManyRequests => "Try again later.",
        }
    }

    pub fn status(self) -> StatusCode {
        match self {
            Self::TooManyRequests => StatusCode::TOO_MANY_REQUESTS,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

#[component]
pub async fn cant_connect(cx: &Cx, reason: CantConnect) -> Result<impl View> {
    Ok(view! {
        (reason.status())
        <section class="l-contain l-skinny">
            page_heading(text: reason.heading())
            <p class="a-lead">(reason.explanation())</p>
        </section>
    })
}
