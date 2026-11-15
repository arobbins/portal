use topcoat::runtime::record;

use super::problem::ActionProblem;

pub const ID_FIELD: &str = "id";
pub const SESSION_EXPIRED: &str = "Your session has expired. Please sign in again.";

#[record]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

impl FieldError {
    pub fn of(problem: impl ActionProblem) -> Self {
        Self {
            field: problem.field().unwrap_or_default().to_string(),
            message: problem.message().to_string(),
        }
    }

    pub fn general(message: &str) -> Self {
        Self {
            field: String::new(),
            message: message.to_string(),
        }
    }

    pub fn session_expired() -> Self {
        Self::general(SESSION_EXPIRED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::problem::TooManyAttempts;

    #[test]
    fn a_field_error_carries_the_problem_field_and_message() {
        assert_eq!(
            FieldError::of(TooManyAttempts),
            FieldError {
                field: String::new(),
                message: TooManyAttempts.message().to_string(),
            }
        );
    }
}
