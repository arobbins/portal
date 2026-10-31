mod account;
mod account_nav;
mod account_profile;
mod account_security;
mod login;
mod profile_menu;
mod register;

pub use account::AccountPage;
pub use login::LoginPage;
pub use register::RegisterPage;

use crate::web::use_globals;

fn redirect_signed_in_users() {
    if use_globals().current_user.is_some() {
        #[cfg(feature = "ssr")]
        leptos_axum::redirect("/account");
    }
}
