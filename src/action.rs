mod browser;
mod caller;
mod field_error;
mod problem;

pub use browser::{browser_caller, require_browser_caller};
pub use caller::{Caller, Channel, Denied};
pub use field_error::{FieldError, ID_FIELD, SESSION_EXPIRED};
pub use problem::{
    ActionError, ActionProblem, ActionResult, Outcome, ProblemSpec, TooManyAttempts, fail,
    page_status,
};

macro_rules! authorize {
    ($check:expr) => {
        if let Err(denied) = $check {
            return Ok(Err($crate::action::ActionError::Denied(denied)));
        }
    };
}
pub(crate) use authorize;
