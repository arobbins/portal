pub const LOGIN_PATH: &str = "/login";
pub const LOGOUT_PATH: &str = "/logout";
pub const REGISTER_PATH: &str = "/register";
pub const ACCOUNT_PATH: &str = "/account";
pub const API_DOCS_PATH: &str = "/docs/api";
pub const RESET_PASSWORD_PATH: &str = "/reset-password";
pub const PRICING_PATH: &str = "/pricing";
pub const CHECKOUT_PAID_PATH: &str = "/account/checkout/paid";
pub const CHECKOUT_CANCELLED_PATH: &str = "/account/checkout/cancelled";

#[cfg(test)]
pub const PAGE_PATHS: [&str; 5] = [
    LOGIN_PATH,
    REGISTER_PATH,
    ACCOUNT_PATH,
    API_DOCS_PATH,
    PRICING_PATH,
];
