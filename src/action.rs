use std::convert::Infallible;
use std::sync::Arc;

use topcoat::{
    Error, Result,
    context::Cx,
    router::{StatusCode, error::see_other, href},
    runtime::record,
};

use crate::api::scope::{ApiScope, grants};
use crate::auth::pages::login::login_page;
use crate::auth::session::current_user;
use crate::user::model::User;

pub type Outcome<T, P> = Result<std::result::Result<T, P>>;
pub type ActionResult<T, P> = Outcome<T, ActionError<P>>;

pub fn fail<T, E>(problem: impl Into<E>) -> Outcome<T, E> {
    Ok(Err(problem.into()))
}

macro_rules! authorize {
    ($check:expr) => {
        if let Err(denied) = $check {
            return Ok(Err($crate::action::ActionError::Denied(denied)));
        }
    };
}
pub(crate) use authorize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Channel {
    Browser,
    ApiToken { id: i32, scopes: Vec<ApiScope> },
}

impl Channel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::ApiToken { .. } => "api_token",
        }
    }

    pub fn api_token_id(&self) -> Option<i32> {
        match self {
            Self::Browser => None,
            Self::ApiToken { id, .. } => Some(*id),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Workspace {
    Personal,
}

#[derive(Debug, Clone)]
pub struct Caller {
    pub user: Arc<User>,
    pub via: Channel,
    pub workspace: Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    MissingScope(ApiScope),
    BrowserOnly,
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
            Self::Denied(denied) => Err(Error::msg(format!(
                "a browser caller was denied: {denied:?}"
            ))),
        }
    }
}

pub trait ActionProblem: Copy {
    fn code(self) -> &'static str;
    fn message(self) -> &'static str;

    fn field(self) -> Option<&'static str> {
        None
    }

    fn status(self) -> StatusCode {
        StatusCode::UNPROCESSABLE_ENTITY
    }
}

impl ActionProblem for Infallible {
    fn code(self) -> &'static str {
        match self {}
    }

    fn message(self) -> &'static str {
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
    fn code(self) -> &'static str {
        "too_many_attempts"
    }

    fn message(self) -> &'static str {
        "Too many attempts. Please wait a few minutes and try again."
    }

    fn status(self) -> StatusCode {
        StatusCode::TOO_MANY_REQUESTS
    }
}

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

pub const ID_FIELD: &str = "id";
pub const SESSION_EXPIRED: &str = "Your session has expired. Please sign in again.";

impl Caller {
    pub fn browser(user: Arc<User>) -> Self {
        Self {
            user,
            via: Channel::Browser,
            workspace: Workspace::Personal,
        }
    }

    pub fn api_token(user: Arc<User>, id: i32, scopes: Vec<ApiScope>) -> Self {
        Self {
            user,
            via: Channel::ApiToken { id, scopes },
            workspace: Workspace::Personal,
        }
    }

    pub fn owner_id(&self) -> i32 {
        match self.workspace {
            Workspace::Personal => self.user.id,
        }
    }

    pub fn require(&self, needed: ApiScope) -> std::result::Result<(), Denied> {
        match &self.via {
            Channel::Browser => Ok(()),
            Channel::ApiToken { scopes, .. } if grants(scopes, needed) => Ok(()),
            Channel::ApiToken { .. } => Err(Denied::MissingScope(needed)),
        }
    }

    pub fn require_browser(&self) -> std::result::Result<(), Denied> {
        match self.via {
            Channel::Browser => Ok(()),
            Channel::ApiToken { .. } => Err(Denied::BrowserOnly),
        }
    }

    pub fn api_token_id(&self) -> Option<i32> {
        self.via.api_token_id()
    }
}

pub fn browser_caller(cx: &Cx) -> Option<Caller> {
    current_user(cx).map(Caller::browser)
}

pub fn require_browser_caller(cx: &Cx) -> Result<Caller> {
    browser_caller(cx).ok_or_else(|| see_other(href!(login_page).resolve(cx)).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller(via: Channel) -> Caller {
        Caller {
            via,
            ..Caller::browser(Arc::new(User::default_for_tests()))
        }
    }

    #[test]
    fn the_browser_has_every_scope() {
        let browser = caller(Channel::Browser);

        assert_eq!(browser.require(ApiScope::ProfileWrite), Ok(()));
        assert_eq!(browser.require_browser(), Ok(()));
        assert_eq!(browser.api_token_id(), None);
        assert_eq!(browser.owner_id(), browser.user.id);
    }

    #[test]
    fn a_token_has_only_its_scopes_and_never_browser_only_actions() {
        let reader = caller(Channel::ApiToken {
            id: 7,
            scopes: vec![ApiScope::ProfileRead],
        });
        let writer = caller(Channel::ApiToken {
            id: 8,
            scopes: vec![ApiScope::ProfileWrite],
        });

        assert_eq!(reader.require(ApiScope::ProfileRead), Ok(()));
        assert_eq!(
            reader.require(ApiScope::ProfileWrite),
            Err(Denied::MissingScope(ApiScope::ProfileWrite))
        );
        assert_eq!(writer.require(ApiScope::ProfileRead), Ok(()));
        assert_eq!(writer.require_browser(), Err(Denied::BrowserOnly));
        assert_eq!(writer.api_token_id(), Some(8));
    }

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
