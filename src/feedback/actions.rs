use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::action::{ActionProblem, ProblemSpec, TooManyAttempts};

mod list;
mod send;

pub use list::{FEEDBACK_SHOWN, FeedbackView, list_api_feedback};
pub use send::send_api_feedback;

pub const MAX_FEEDBACK_LENGTH: usize = 2000;
const MAX_REFERENCE_LENGTH: usize = 100;
pub const MESSAGE_FIELD: &str = "message";
pub const ENDPOINT_FIELD: &str = "endpoint";
pub const PROBLEM_CODE_FIELD: &str = "problem_code";
pub const REQUEST_ID_FIELD: &str = "request_id";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum FeedbackCategory {
    ConfusingDocs,
    UnexpectedError,
    MissingCapability,
    WrongSchema,
    Other,
}

impl FeedbackCategory {
    #[cfg(test)]
    pub const ALL: [Self; 5] = [
        Self::ConfusingDocs,
        Self::UnexpectedError,
        Self::MissingCapability,
        Self::WrongSchema,
        Self::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfusingDocs => "confusing_docs",
            Self::UnexpectedError => "unexpected_error",
            Self::MissingCapability => "missing_capability",
            Self::WrongSchema => "wrong_schema",
            Self::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ConfusingDocs => "Confusing docs",
            Self::UnexpectedError => "Unexpected error",
            Self::MissingCapability => "Missing capability",
            Self::WrongSchema => "Wrong schema",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    description = "Tell the people who run this API what got in your way, so they can fix the API or its docs. Send it when a call failed in a way the docs didn't prepare you for, a field or error was unclear, or something you needed doesn't exist. Never include tokens or other secrets."
)]
pub struct SendApiFeedback {
    #[schemars(description = "What kind of problem this is.")]
    pub category: FeedbackCategory,
    #[serde(default)]
    #[schemars(
        description = "The endpoint it's about: its operation id or MCP tool name (`update-profile` or `update_profile`).",
        length(max = MAX_REFERENCE_LENGTH)
    )]
    pub endpoint: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "The `code` of the error you got, if any (`name_too_long`).",
        length(max = MAX_REFERENCE_LENGTH)
    )]
    pub problem_code: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "The `x-request-id` response header of the call it's about, if you have it.",
        length(max = MAX_REFERENCE_LENGTH)
    )]
    pub request_id: Option<String>,
    #[schemars(
        description = "What you tried, what you expected and what happened instead.",
        length(min = 1, max = MAX_FEEDBACK_LENGTH)
    )]
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[schemars(description = "Confirmation that the feedback was received.")]
pub struct FeedbackReceipt {
    #[schemars(description = "The feedback's id (fbk_…), to refer to it later.")]
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackProblem {
    MessageMissing,
    MessageTooLong,
    UnknownEndpoint,
    UnknownProblemCode,
    InvalidRequestId,
    TooManyAttempts,
}

impl FeedbackProblem {
    pub const ALL: [Self; 6] = [
        Self::MessageMissing,
        Self::MessageTooLong,
        Self::UnknownEndpoint,
        Self::UnknownProblemCode,
        Self::InvalidRequestId,
        Self::TooManyAttempts,
    ];
}

impl ActionProblem for FeedbackProblem {
    fn spec(self) -> ProblemSpec {
        match self {
            Self::MessageMissing => {
                ProblemSpec::new("feedback_message_missing", "Say what got in your way.")
                    .on_field(MESSAGE_FIELD)
            }
            Self::MessageTooLong => ProblemSpec::new(
                "feedback_message_too_long",
                "Feedback can be at most 2000 characters.",
            )
            .on_field(MESSAGE_FIELD),
            Self::UnknownEndpoint => ProblemSpec::new(
                "unknown_endpoint",
                "That isn't an endpoint of this API. Use an operation id from the OpenAPI document or an MCP tool name.",
            )
            .on_field(ENDPOINT_FIELD),
            Self::UnknownProblemCode => ProblemSpec::new(
                "unknown_problem_code",
                "That isn't an error code this API returns. Copy the `code` from the error response.",
            )
            .on_field(PROBLEM_CODE_FIELD),
            Self::InvalidRequestId => ProblemSpec::new(
                "invalid_request_id",
                "A request id is the UUID in a response's `x-request-id` header.",
            )
            .on_field(REQUEST_ID_FIELD),
            Self::TooManyAttempts => TooManyAttempts.spec(),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod limits {
    use super::*;

    #[test]
    fn the_too_long_message_matches_the_limit() {
        assert!(
            FeedbackProblem::MessageTooLong
                .message()
                .contains(&MAX_FEEDBACK_LENGTH.to_string())
        );
    }
}
