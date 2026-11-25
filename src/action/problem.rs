use std::convert::Infallible;

use topcoat::{Result, router::StatusCode};

use super::caller::Denied;

pub type Outcome<T, P> = Result<std::result::Result<T, P>>;
pub type ActionResult<T, P> = Outcome<T, ActionError<P>>;

pub fn fail<T, E>(problem: impl Into<E>) -> Outcome<T, E> {
    Ok(Err(problem.into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionError<P> {
    Denied(Denied),
    Problem(P),
}

impl<P> From<P> for ActionError<P> {
    fn from(problem: P) -> Self {
        Self::Problem(problem)
    }
}

impl<P> ActionError<P> {
    pub fn for_browser(self) -> Result<P> {
        match self {
            Self::Problem(problem) => Ok(problem),
            Self::Denied(denied) => Err(topcoat::Error::msg(format!(
                "a browser caller was denied: {denied:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProblemSpec {
    pub code: &'static str,
    pub message: &'static str,
    pub field: Option<&'static str>,
    pub status: StatusCode,
}

impl ProblemSpec {
    pub const fn new(code: &'static str, message: &'static str) -> Self {
        Self {
            code,
            message,
            field: None,
            status: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    pub const fn on_field(mut self, field: &'static str) -> Self {
        self.field = Some(field);
        self
    }

    pub const fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }
}

pub trait ActionProblem: Copy {
    fn spec(self) -> ProblemSpec;

    fn code(self) -> &'static str {
        self.spec().code
    }

    fn message(self) -> &'static str {
        self.spec().message
    }

    fn field(self) -> Option<&'static str> {
        self.spec().field
    }

    fn status(self) -> StatusCode {
        self.spec().status
    }
}

impl ActionProblem for ProblemSpec {
    fn spec(self) -> ProblemSpec {
        self
    }
}

impl ActionProblem for Infallible {
    fn spec(self) -> ProblemSpec {
        match self {}
    }
}

pub fn page_status(problem: impl ActionProblem) -> StatusCode {
    match problem.status() {
        StatusCode::TOO_MANY_REQUESTS => StatusCode::TOO_MANY_REQUESTS,
        _ => StatusCode::OK,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooManyAttempts;

impl ActionProblem for TooManyAttempts {
    fn spec(self) -> ProblemSpec {
        ProblemSpec::new(
            "too_many_attempts",
            "Too many attempts. Please wait a few minutes and try again.",
        )
        .with_status(StatusCode::TOO_MANY_REQUESTS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_spec_defaults_to_an_unprocessable_problem_without_a_field() {
        let spec = ProblemSpec::new("code", "Message.");

        assert_eq!(spec.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(spec.field, None);
        assert_eq!(spec.on_field("name").field, Some("name"));
    }

    #[test]
    fn only_rate_limits_change_a_pages_status() {
        assert_eq!(page_status(TooManyAttempts), StatusCode::TOO_MANY_REQUESTS);
    }
}
