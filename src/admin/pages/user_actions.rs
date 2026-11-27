use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, attributes, component, view},
};

use super::user_routes::{
    delete_user_submit, disable_user_submit, enable_user_submit, resend_verification_submit,
    sign_out_user_submit,
};
use crate::action::ID_FIELD;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{CardHeadingLevel, card, card_body, card_header};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::property_list::{property, property_list};
use crate::user::model::{AccountStatus, Role, User};

struct AdminAction {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    question: &'static str,
    action: String,
    destructive: bool,
}

fn available_actions(cx: &Cx, user: &User) -> Vec<AdminAction> {
    let status = user.status();
    let mut actions = Vec::new();
    if status == AccountStatus::AwaitingPassword {
        actions.push(AdminAction {
            id: "resend-verification",
            label: "Resend confirmation email",
            description: "Send a new link so they can confirm their email and set a password.",
            question: "Send a new confirmation email to this user?",
            action: href!(resend_verification_submit).resolve(cx),
            destructive: false,
        });
    }
    actions.push(AdminAction {
        id: "sign-out-user",
        label: "Sign out everywhere",
        description: "End every session on every device. They can sign in again.",
        question: "Sign this user out of every device?",
        action: href!(sign_out_user_submit).resolve(cx),
        destructive: false,
    });
    actions.push(access_action(cx, status));
    actions.push(AdminAction {
        id: "delete-user",
        label: "Delete account",
        description: "Delete the account and everything in it. This can't be undone.",
        question: "Delete this account and everything in it? This can't be undone.",
        action: href!(delete_user_submit).resolve(cx),
        destructive: true,
    });
    actions
}

fn access_action(cx: &Cx, status: AccountStatus<'_>) -> AdminAction {
    if matches!(status, AccountStatus::Disabled { .. }) {
        return AdminAction {
            id: "enable-user",
            label: "Enable account",
            description: "Let them sign in again. Their passkeys and API tokens work again too.",
            question: "Enable this account again?",
            action: href!(enable_user_submit).resolve(cx),
            destructive: false,
        };
    }
    AdminAction {
        id: "disable-user",
        label: "Disable account",
        description: "Block every sign-in and API token until you enable it again.",
        question: "Disable this account? They'll be signed out everywhere.",
        action: href!(disable_user_submit).resolve(cx),
        destructive: false,
    }
}

#[component]
pub async fn user_actions_card(cx: &Cx, user: &User, is_self: bool) -> Result<impl View> {
    let locked = is_self || user.role == Role::Admin;
    let actions = if locked {
        Vec::new()
    } else {
        available_actions(cx, user)
    };
    let reason = if is_self {
        "This is your account. Change it from your account settings."
    } else {
        "Admins can't be changed here. Remove their admin role on the server first."
    };

    Ok(view! {
        card(
            card_header(title: "Actions", level: CardHeadingLevel::H3)
            card_body(
                if locked {
                    <p>(reason)</p>
                } else {
                    property_list(
                        #[key(action.id)]
                        for action in &actions {
                            property(
                                name: action.label,
                                description: Some(action.description),
                                button(
                                    variant: if action.destructive {
                                        ButtonVariant::Danger
                                    } else {
                                        ButtonVariant::Secondary
                                    },
                                    size: ButtonSize::Sm,
                                    attrs: attributes! { command="show-modal" commandfor=(action.id) },
                                    (action.label)
                                )
                            )
                        }
                    )
                }
            )
        )
        #[key(action.id)]
        for action in &actions {
            confirm_dialog(
                id: action.id,
                heading: action.label,
                confirm_label: action.label,
                action: Some(action.action.as_str()),
                <input type="hidden" name=(ID_FIELD) value=(user.public_id.as_str())>
                <p>(action.question)</p>
            )
        }
    })
}
