use super::{Expect, Labeled, Machine, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    None,
    Connected,
    Suspended,
    Disconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionEvent {
    Connect,
    Refresh,
    ReuseRefreshToken,
    AppRevokes,
    OwnerDisconnects,
    PasswordReset,
    SignOutOtherDevices,
    AdminSignsOutEverywhere,
    AdminDisables,
    AdminEnables,
    AccountDeleted,
}

use ConnectionEvent as E;
use ConnectionState as S;

pub const CONNECTION: Machine<ConnectionState, ConnectionEvent> = Machine {
    name: "Connection",
    summary: "An app the owner connected through OAuth: its connection row and the access and refresh tokens it calls the MCP endpoint with.",
    states: &[S::None, S::Connected, S::Suspended, S::Disconnected],
    terminal: &[],
    events: &[
        E::Connect,
        E::Refresh,
        E::ReuseRefreshToken,
        E::AppRevokes,
        E::OwnerDisconnects,
        E::PasswordReset,
        E::SignOutOtherDevices,
        E::AdminSignsOutEverywhere,
        E::AdminDisables,
        E::AdminEnables,
        E::AccountDeleted,
    ],
    stored_in: &[
        "oauth_connections (row)",
        "oauth_tokens (row)",
        "oauth_tokens.replaced_at",
        "users.disabled_at",
    ],
    rules: &[
        APP_GETS_IN_ONLY_WHILE_CONNECTED,
        REFRESH_REPLACES_THE_ACCESS_TOKEN,
        CHANGES_ARE_RECORDED,
        ONLY_HASHES_ARE_STORED,
    ],
    writers: &[
        writer(CONNECT, "query_upsert_connection", &[E::Connect]),
        writer(STORE, "query_insert_token", &[E::Connect, E::Refresh]),
        writer(
            STORE,
            "query_delete_access_tokens",
            &[E::Connect, E::Refresh],
        ),
        writer(REFRESH, "query_replace_refresh_token", &[E::Refresh]),
        writer(
            END,
            "query_delete_connection_by_id",
            &[E::ReuseRefreshToken, E::AppRevokes],
        ),
        writer(
            "src/oauth/actions/manage.rs:disconnect_app",
            "query_delete_connection",
            &[E::OwnerDisconnects],
        ),
        writer(
            "src/oauth/lifecycle.rs:disconnect_all_for_user",
            "query_delete_user_connections",
            &[
                E::PasswordReset,
                E::SignOutOtherDevices,
                E::AdminSignsOutEverywhere,
            ],
        ),
        writer(
            "src/oauth/lifecycle.rs:delete_expired",
            "query_delete_expired_tokens",
            &[],
        ),
        writer(
            "src/admin/actions/manage.rs:disable_user",
            "query_disable_user",
            &[E::AdminDisables],
        ),
        writer(
            "src/admin/actions/manage.rs:enable_user",
            "query_enable_user",
            &[E::AdminEnables],
        ),
    ],
};

const CONNECT: &str = "src/oauth/actions/connect.rs:store_connection";
const STORE: &str = "src/oauth/grant.rs:store_tokens";
const REFRESH: &str = "src/oauth/grant.rs:refresh";
const END: &str = "src/oauth/grant.rs:end_connection";

pub const APP_GETS_IN_ONLY_WHILE_CONNECTED: &str = "The app's newest access token gets in only while the connection is live and the owner enabled.";
pub const REFRESH_REPLACES_THE_ACCESS_TOKEN: &str =
    "A refresh replaces both tokens: the old access token stops working at once.";
pub const CHANGES_ARE_RECORDED: &str = "Connecting records app_connected and emails the owner; the owner, the app's revoke or a reused refresh token record app_disconnected; account-wide sign-outs record their own event instead.";
pub const ONLY_HASHES_ARE_STORED: &str = "Only hashes of the access and refresh tokens are stored.";

const NO_TOKENS: &str = "The app has no tokens before it connects.";
const NO_SESSION: &str = "A disabled owner can't sign in to approve or manage apps.";

const fn writer(
    caller: &'static str,
    query: &'static str,
    events: &'static [ConnectionEvent],
) -> Writer<ConnectionEvent> {
    Writer {
        caller,
        query,
        events,
    }
}

pub fn expect(from: ConnectionState, event: ConnectionEvent) -> Expect<ConnectionState> {
    use Expect::{Moves, Refused, Unreachable};
    match (from, event) {
        (S::None, E::Refresh | E::ReuseRefreshToken | E::AppRevokes) => Unreachable(NO_TOKENS),
        (S::Suspended, E::Connect | E::OwnerDisconnects | E::SignOutOtherDevices) => {
            Unreachable(NO_SESSION)
        }
        (_, E::Connect) => Moves(S::Connected),
        (S::Connected, E::Refresh) => Moves(S::Connected),
        (S::Connected, E::AdminDisables) => Moves(S::Suspended),
        (S::Suspended, E::AdminEnables) => Moves(S::Connected),
        (S::Connected, E::AdminEnables) => Refused,
        (S::Suspended, E::Refresh | E::AdminDisables | E::PasswordReset) => Refused,
        (S::Connected | S::Suspended, _) => Moves(S::Disconnected),
        (S::None | S::Disconnected, _) => Refused,
    }
}

impl Labeled for ConnectionState {
    fn id(self) -> &'static str {
        match self {
            S::None => "none",
            S::Connected => "connected",
            S::Suspended => "suspended",
            S::Disconnected => "disconnected",
        }
    }

    fn label(self) -> &'static str {
        match self {
            S::None => "Not connected",
            S::Connected => "Connected",
            S::Suspended => "Owner disabled",
            S::Disconnected => "Disconnected",
        }
    }
}

impl Labeled for ConnectionEvent {
    fn id(self) -> &'static str {
        match self {
            E::Connect => "connect",
            E::Refresh => "refresh",
            E::ReuseRefreshToken => "reuse_refresh_token",
            E::AppRevokes => "app_revokes",
            E::OwnerDisconnects => "owner_disconnects",
            E::PasswordReset => "password_reset",
            E::SignOutOtherDevices => "sign_out_other_devices",
            E::AdminSignsOutEverywhere => "admin_signs_out_everywhere",
            E::AdminDisables => "admin_disables",
            E::AdminEnables => "admin_enables",
            E::AccountDeleted => "account_deleted",
        }
    }

    fn label(self) -> &'static str {
        match self {
            E::Connect => "Owner approves the app, which takes its code",
            E::Refresh => "The app refreshes its tokens",
            E::ReuseRefreshToken => "A replaced refresh token is used again",
            E::AppRevokes => "The app calls /oauth/revoke",
            E::OwnerDisconnects => "Owner disconnects it",
            E::PasswordReset => "Password reset by email link",
            E::SignOutOtherDevices => "Owner signs out other devices",
            E::AdminSignsOutEverywhere => "Admin signs the owner out everywhere",
            E::AdminDisables => "Admin disables the owner",
            E::AdminEnables => "Admin enables the owner",
            E::AccountDeleted => "Owner's account deleted",
        }
    }
}

#[cfg(test)]
mod tests;
