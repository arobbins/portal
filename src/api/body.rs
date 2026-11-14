use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, StatusCode, request,
        request::{Bytes, FromRequest},
    },
};

use crate::action::ActionProblem;
use crate::api::problem::{ApiProblem, ApiResponse};

pub const MAX_JSON_BODY_KB: usize = 64;
pub const MAX_JSON_BODY: usize = MAX_JSON_BODY_KB * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyProblem {
    NotJson,
    TooLarge,
    InvalidJson,
}

impl BodyProblem {
    pub const ALL: [Self; 3] = [Self::NotJson, Self::TooLarge, Self::InvalidJson];
}

impl ActionProblem for BodyProblem {
    fn code(self) -> &'static str {
        match self {
            Self::NotJson => "unsupported_media_type",
            Self::TooLarge => "body_too_large",
            Self::InvalidJson => "invalid_json",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::NotJson => "Send the request body as JSON with Content-Type: application/json.",
            Self::TooLarge => "The request body is too large. JSON bodies can be at most 64 KB.",
            Self::InvalidJson => {
                "The request body isn't valid for this endpoint: it must be JSON with only the documented fields."
            }
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::NotJson => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::TooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            Self::InvalidJson => StatusCode::BAD_REQUEST,
        }
    }
}

pub async fn read_json<T: DeserializeOwned>(
    cx: &Cx,
    body: Body,
) -> Result<std::result::Result<T, ApiResponse>> {
    let headers = request::headers(cx);
    let is_json = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if !is_json {
        return Ok(Err(ApiProblem::of(BodyProblem::NotJson).response()));
    }

    let declared_length = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    if declared_length.is_some_and(|length| length > MAX_JSON_BODY) {
        return Ok(Err(ApiProblem::of(BodyProblem::TooLarge).response()));
    }

    let Ok(bytes) = Bytes::from_request(cx, body).await else {
        return Ok(Err(ApiProblem::of(BodyProblem::TooLarge).response()));
    };

    Ok(serde_json::from_slice(&bytes).map_err(|error| {
        ApiProblem::of(BodyProblem::InvalidJson)
            .detail(format!(
                "{} Details: {error}.",
                BodyProblem::InvalidJson.message()
            ))
            .response()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_size_message_matches_the_limit() {
        assert!(
            BodyProblem::TooLarge
                .message()
                .contains(&format!("{MAX_JSON_BODY_KB} KB"))
        );
    }
}
