mod change_password;
mod link_password;
mod password_reset;
mod register;
mod sign_in;

pub use change_password::{ChangePassword, ChangePasswordProblem, change_password};
pub use link_password::{LinkPasswordProblem, NewPassword, link_is_valid, set_password_from_link};
pub use password_reset::{
    PasswordResetJob, RequestPasswordReset, ResetRequestProblem, request_password_reset,
};
pub use register::{Register, RegisterProblem, register};
pub use sign_in::{SignIn, SignInProblem, sign_in_with_password};

pub const EMAIL_FIELD: &str = "email";
const INVALID_EMAIL: &str = "Please enter a valid email address.";
