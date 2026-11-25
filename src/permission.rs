use crate::api::scope::ApiScope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ViewProfile,
    EditProfile,
    ManagePassword,
    ManageEmail,
    ConfirmIdentity,
    ManagePasskeys,
    ManageTwoFactor,
    ManageDevices,
    ViewActivity,
    ManageApiTokens,
    DeleteAccount,
    ExportData,
    AdministerUsers,
    SendApiFeedback,
    ViewPlans,
    ViewBilling,
    StartCheckout,
    ManageBilling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenAccess {
    BrowserOnly,
    Scope(ApiScope),
    AnyToken,
    Public,
}

impl Permission {
    pub const ALL: [Self; 18] = [
        Self::ViewProfile,
        Self::EditProfile,
        Self::ManagePassword,
        Self::ManageEmail,
        Self::ConfirmIdentity,
        Self::ManagePasskeys,
        Self::ManageTwoFactor,
        Self::ManageDevices,
        Self::ViewActivity,
        Self::ManageApiTokens,
        Self::DeleteAccount,
        Self::ExportData,
        Self::AdministerUsers,
        Self::SendApiFeedback,
        Self::ViewPlans,
        Self::ViewBilling,
        Self::StartCheckout,
        Self::ManageBilling,
    ];

    pub fn token_access(self) -> TokenAccess {
        match self {
            Self::ViewProfile => TokenAccess::Scope(ApiScope::ProfileRead),
            Self::EditProfile => TokenAccess::Scope(ApiScope::ProfileWrite),
            Self::SendApiFeedback => TokenAccess::AnyToken,
            Self::ViewPlans => TokenAccess::Public,
            Self::ViewBilling => TokenAccess::Scope(ApiScope::BillingRead),
            Self::StartCheckout => TokenAccess::Scope(ApiScope::BillingWrite),
            Self::ManagePassword
            | Self::ManageEmail
            | Self::ConfirmIdentity
            | Self::ManagePasskeys
            | Self::ManageTwoFactor
            | Self::ManageDevices
            | Self::ViewActivity
            | Self::ManageApiTokens
            | Self::DeleteAccount
            | Self::ExportData
            | Self::AdministerUsers
            | Self::ManageBilling => TokenAccess::BrowserOnly,
        }
    }

    pub fn admins_only(self) -> bool {
        matches!(self, Self::AdministerUsers)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRole {
    Owner,
}

impl WorkspaceRole {
    pub fn grants(self, permission: Permission) -> bool {
        match self {
            Self::Owner => Permission::ALL.contains(&permission),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_settings_are_browser_only() {
        for permission in [
            Permission::ManagePassword,
            Permission::ManageEmail,
            Permission::ConfirmIdentity,
            Permission::ManagePasskeys,
            Permission::ManageTwoFactor,
            Permission::ManageDevices,
            Permission::ViewActivity,
            Permission::ManageApiTokens,
            Permission::DeleteAccount,
            Permission::ExportData,
            Permission::AdministerUsers,
            Permission::ManageBilling,
        ] {
            assert_eq!(permission.token_access(), TokenAccess::BrowserOnly);
        }
    }

    #[test]
    fn any_token_may_send_api_feedback() {
        assert_eq!(
            Permission::SendApiFeedback.token_access(),
            TokenAccess::AnyToken
        );
    }

    #[test]
    fn anyone_may_see_the_plans() {
        assert_eq!(Permission::ViewPlans.token_access(), TokenAccess::Public);
    }

    #[test]
    fn every_api_scope_belongs_to_a_permission() {
        for scope in ApiScope::ALL {
            assert!(
                Permission::ALL
                    .into_iter()
                    .any(|permission| permission.token_access() == TokenAccess::Scope(scope)),
                "{}",
                scope.as_str()
            );
        }
    }

    #[test]
    fn the_owner_can_do_everything() {
        for permission in Permission::ALL {
            assert!(WorkspaceRole::Owner.grants(permission));
        }
    }
}
