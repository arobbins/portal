use std::sync::Arc;

use crate::api::scope::{ApiScope, grants};
use crate::permission::{Permission, TokenAccess, WorkspaceRole};
use crate::user::model::{Role, User};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Channel {
    Browser,
    ApiToken { id: i32, scopes: Vec<ApiScope> },
}

impl Channel {
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
    NotPermitted(Permission),
    MissingScope(ApiScope),
    BrowserOnly,
}

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

    pub fn role(&self) -> WorkspaceRole {
        match self.workspace {
            Workspace::Personal => WorkspaceRole::Owner,
        }
    }

    pub fn require(&self, permission: Permission) -> std::result::Result<(), Denied> {
        if !self.role().grants(permission) {
            return Err(Denied::NotPermitted(permission));
        }
        if permission.admins_only() && self.user.role != Role::Admin {
            return Err(Denied::NotPermitted(permission));
        }
        match (&self.via, permission.token_access()) {
            (Channel::Browser, _)
            | (Channel::ApiToken { .. }, TokenAccess::AnyToken | TokenAccess::Public) => Ok(()),
            (Channel::ApiToken { .. }, TokenAccess::BrowserOnly) => Err(Denied::BrowserOnly),
            (Channel::ApiToken { scopes, .. }, TokenAccess::Scope(scope))
                if grants(scopes, scope) =>
            {
                Ok(())
            }
            (Channel::ApiToken { .. }, TokenAccess::Scope(scope)) => {
                Err(Denied::MissingScope(scope))
            }
        }
    }

    pub fn api_token_id(&self) -> Option<i32> {
        self.via.api_token_id()
    }
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

        for permission in Permission::ALL.into_iter().filter(|p| !p.admins_only()) {
            assert_eq!(browser.require(permission), Ok(()));
        }
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

        assert_eq!(reader.require(Permission::ViewProfile), Ok(()));
        assert_eq!(
            reader.require(Permission::EditProfile),
            Err(Denied::MissingScope(ApiScope::ProfileWrite))
        );
        assert_eq!(writer.require(Permission::ViewProfile), Ok(()));
        assert_eq!(
            writer.require(Permission::ManageApiTokens),
            Err(Denied::BrowserOnly)
        );
        assert_eq!(writer.api_token_id(), Some(8));
    }

    #[test]
    fn a_token_without_scopes_can_still_send_api_feedback() {
        let bare = caller(Channel::ApiToken {
            id: 9,
            scopes: Vec::new(),
        });

        assert_eq!(bare.require(Permission::SendApiFeedback), Ok(()));
        assert_eq!(
            bare.require(Permission::ViewProfile),
            Err(Denied::MissingScope(ApiScope::ProfileRead))
        );
    }

    #[test]
    fn only_admins_may_administer_users() {
        let subscriber = caller(Channel::Browser);
        let admin = Caller::browser(Arc::new(User {
            role: Role::Admin,
            ..User::default_for_tests()
        }));
        let admin_token = Caller {
            via: Channel::ApiToken {
                id: 1,
                scopes: ApiScope::ALL.to_vec(),
            },
            ..admin.clone()
        };

        assert_eq!(
            subscriber.require(Permission::AdministerUsers),
            Err(Denied::NotPermitted(Permission::AdministerUsers))
        );
        assert_eq!(admin.require(Permission::AdministerUsers), Ok(()));
        assert_eq!(
            admin_token.require(Permission::AdministerUsers),
            Err(Denied::BrowserOnly)
        );
    }
}
