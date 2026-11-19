use super::caller::{CallerState, expect as caller_decision};
use super::{Expect, Labeled, Machine};
use crate::permission::Permission;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionState {
    Asked(Permission),
    Decided(CallerState),
}

use CallerState as C;
use Permission as P;
use PermissionState::{Asked, Decided};

pub const PERMISSION: Machine<PermissionState, CallerState> = Machine {
    name: "Permission",
    summary: "Each permission and what every kind of caller gets when it asks for it: the Caller table seen from the permission's side, plus rules about the permissions themselves.",
    states: &[
        Asked(P::ViewProfile),
        Asked(P::EditProfile),
        Asked(P::ManagePassword),
        Asked(P::ManageEmail),
        Asked(P::ConfirmIdentity),
        Asked(P::ManagePasskeys),
        Asked(P::ManageTwoFactor),
        Asked(P::ManageDevices),
        Asked(P::ViewActivity),
        Asked(P::ManageApiTokens),
        Asked(P::DeleteAccount),
        Asked(P::ExportData),
        Asked(P::AdministerUsers),
        Asked(P::SendApiFeedback),
        Decided(C::Allowed),
        Decided(C::NeedsBrowser),
        Decided(C::MissingScope),
        Decided(C::NotPermitted),
    ],
    terminal: &[
        Decided(C::Allowed),
        Decided(C::NeedsBrowser),
        Decided(C::MissingScope),
        Decided(C::NotPermitted),
    ],
    events: &[
        C::SubscriberBrowser,
        C::AdminBrowser,
        C::ReadToken,
        C::WriteToken,
        C::AdminWriteToken,
    ],
    stored_in: &[],
    rules: &[
        SENSITIVE_ARE_BROWSER_ONLY,
        ONLY_ADMINISTERING_IS_ADMIN_ONLY,
        TOKEN_PERMISSIONS_HAVE_ENDPOINTS,
    ],
    writers: &[],
};

pub const SENSITIVE: [Permission; 11] = [
    P::ManagePassword,
    P::ManageEmail,
    P::ConfirmIdentity,
    P::ManagePasskeys,
    P::ManageTwoFactor,
    P::ManageDevices,
    P::ViewActivity,
    P::ManageApiTokens,
    P::DeleteAccount,
    P::ExportData,
    P::AdministerUsers,
];

pub const SENSITIVE_ARE_BROWSER_ONLY: &str = "Exactly the security-sensitive permissions (password, email, identity, passkeys, two-factor, devices, activity, tokens, deleting or exporting the account, administering users) are browser-only.";
pub const ONLY_ADMINISTERING_IS_ADMIN_ONLY: &str =
    "Administering users is the only permission limited to admins.";
pub const TOKEN_PERMISSIONS_HAVE_ENDPOINTS: &str =
    "Every permission a token can use is reachable through at least one API endpoint.";

const A_DECISION: &str = "A decision, not a permission.";

pub fn expect(from: PermissionState, caller: CallerState) -> Expect<PermissionState> {
    let Asked(permission) = from else {
        return Expect::Unreachable(A_DECISION);
    };
    match caller_decision(caller, permission) {
        Expect::Moves(decision) => Expect::Moves(Decided(decision)),
        Expect::Refused => Expect::Refused,
        Expect::Unreachable(reason) => Expect::Unreachable(reason),
    }
}

impl Labeled for PermissionState {
    fn id(self) -> &'static str {
        match self {
            Asked(permission) => permission.id(),
            Decided(decision) => decision.id(),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Asked(permission) => permission.label(),
            Decided(decision) => decision.label(),
        }
    }
}

#[cfg(test)]
mod tests;
