mod change_email;
mod change_password;
mod confirm_email;
mod delete_account;
mod link_password;
mod password_age;
mod password_reset;
mod register;
mod sign_in;

pub use change_email::{
    ChangeEmail, ChangeEmailProblem, EmailChangeJob, NEW_EMAIL_FIELD, request_email_change,
};
pub use change_password::{ChangePassword, ChangePasswordProblem, change_password};
pub use confirm_email::{ConfirmEmailProblem, confirm_email_change, pending_email_change};
pub use delete_account::{CONFIRMATION_FIELD, DeleteAccount, DeleteAccountProblem, delete_account};
pub use link_password::{
    LinkPasswordProblem, NewPassword, link_is_valid, link_payload, set_password_from_link,
};
pub use password_age::{PasswordAge, password_age};
pub use password_reset::{
    PasswordResetJob, RequestPasswordReset, ResetRequestProblem, request_password_reset,
};
pub use register::{Register, RegisterProblem, register};
pub use sign_in::{AccountDisabled, SignIn, SignInProblem, sign_in_with_password};

pub const EMAIL_FIELD: &str = "email";
const INVALID_EMAIL: &str = "Please enter a valid email address.";
