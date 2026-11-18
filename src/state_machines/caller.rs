use super::{Expect, Labeled, Machine};
use crate::permission::Permission;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerState {
    SubscriberBrowser,
    AdminBrowser,
    ReadToken,
    WriteToken,
    AdminWriteToken,
    Allowed,
    NeedsBrowser,
    MissingScope,
    NotPermitted,
}

use CallerState as S;
use Permission as P;

pub const CALLER: Machine<CallerState, Permission> = Machine {
    name: "Caller",
    summary: "Who a request comes from (a browser or an API token, a subscriber or an admin) and what each kind of caller is allowed to do.",
    states: &[
        S::SubscriberBrowser,
        S::AdminBrowser,
        S::ReadToken,
        S::WriteToken,
        S::AdminWriteToken,
        S::Allowed,
        S::NeedsBrowser,
        S::MissingScope,
        S::NotPermitted,
    ],
    terminal: &[
        S::Allowed,
        S::NeedsBrowser,
        S::MissingScope,
        S::NotPermitted,
    ],
    events: &Permission::ALL,
    stored_in: &[],
    rules: &[ENDPOINTS_AGREE, BROWSER_ONLY_HAS_NO_ENDPOINT],
    writers: &[],
};

pub const ENDPOINTS_AGREE: &str = "Every API endpoint answers a token exactly as the table says: served when allowed, 403 otherwise.";
pub const BROWSER_ONLY_HAS_NO_ENDPOINT: &str =
    "A permission that needs a browser has no API endpoint or MCP tool at all.";

const A_DECISION: &str = "A decision, not a caller.";

pub fn expect(from: CallerState, permission: Permission) -> Expect<CallerState> {
    use Expect::{Moves, Unreachable};
    let decision = match (from, permission) {
        (S::Allowed | S::NeedsBrowser | S::MissingScope | S::NotPermitted, _) => {
            return Unreachable(A_DECISION);
        }
        (S::AdminBrowser, _) => S::Allowed,
        (S::SubscriberBrowser | S::ReadToken | S::WriteToken, P::AdministerUsers) => {
            S::NotPermitted
        }
        (S::SubscriberBrowser, _) => S::Allowed,
        (S::ReadToken | S::WriteToken | S::AdminWriteToken, P::ViewProfile) => S::Allowed,
        (S::ReadToken, P::EditProfile) => S::MissingScope,
        (S::WriteToken | S::AdminWriteToken, P::EditProfile) => S::Allowed,
        (S::ReadToken | S::WriteToken | S::AdminWriteToken, _) => S::NeedsBrowser,
    };
    Moves(decision)
}

impl Labeled for CallerState {
    fn id(self) -> &'static str {
        match self {
            S::SubscriberBrowser => "subscriber_browser",
            S::AdminBrowser => "admin_browser",
            S::ReadToken => "read_token",
            S::WriteToken => "write_token",
            S::AdminWriteToken => "admin_write_token",
            S::Allowed => "allowed",
            S::NeedsBrowser => "needs_browser",
            S::MissingScope => "missing_scope",
            S::NotPermitted => "not_permitted",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::SubscriberBrowser => "Subscriber in a browser",
            S::AdminBrowser => "Admin in a browser",
            S::ReadToken => "Token with profile:read",
            S::WriteToken => "Token with profile:write",
            S::AdminWriteToken => "Admin's token with profile:write",
            S::Allowed => "Allowed",
            S::NeedsBrowser => "Denied: browser only",
            S::MissingScope => "Denied: missing scope",
            S::NotPermitted => "Denied: role",
        }
    }
}

impl Labeled for Permission {
    fn id(self) -> &'static str {
        match self {
            P::ViewProfile => "view_profile",
            P::EditProfile => "edit_profile",
            P::ManagePassword => "manage_password",
            P::ManageEmail => "manage_email",
            P::ConfirmIdentity => "confirm_identity",
            P::ManagePasskeys => "manage_passkeys",
            P::ManageTwoFactor => "manage_two_factor",
            P::ManageDevices => "manage_devices",
            P::ViewActivity => "view_activity",
            P::ManageApiTokens => "manage_api_tokens",
            P::DeleteAccount => "delete_account",
            P::AdministerUsers => "administer_users",
        }
    }

    fn label(self) -> &'static str {
        match self {
            P::ViewProfile => "View profile",
            P::EditProfile => "Edit profile",
            P::ManagePassword => "Manage password",
            P::ManageEmail => "Manage email",
            P::ConfirmIdentity => "Confirm identity",
            P::ManagePasskeys => "Manage passkeys",
            P::ManageTwoFactor => "Manage two-factor authentication",
            P::ManageDevices => "Manage devices",
            P::ViewActivity => "View activity",
            P::ManageApiTokens => "Manage API tokens",
            P::DeleteAccount => "Delete account",
            P::AdministerUsers => "Administer users",
        }
    }
}

#[cfg(test)]
mod tests;
