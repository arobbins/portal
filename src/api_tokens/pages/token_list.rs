use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, attributes, component, view},
};

use super::create_token_dialog::CREATE_MODAL;
use super::token_fields::TokenDrafts;
use super::token_routes::revoke_api_token_submit;
use super::token_view::TokenRow;
use crate::account::tabs::AccountSection;
use crate::action::ID_FIELD;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::copy_field::copy_field;
use crate::components::empty_state::empty_state;
use crate::components::form::DialogForm;
use crate::components::icon::{IconName, icon};
use crate::components::notice::notice;
use crate::components::property_list::{property, property_list};
use crate::components::tone::Tone;
use crate::routes::API_DOCS_PATH;

const REVOKE_MODAL: &str = "revoke-api-token";

#[component]
pub(super) async fn new_token_card(cx: &Cx, token: &str) -> Result<impl View> {
    Ok(view! {
        card(
            id: Some("new-api-token"),
            card_header(title: "Your new API token")
            card_body(
                notice(
                    tone: Tone::Warning,
                    "Copy this token now. You won't be able to see it again."
                )
                copy_field(label: "API token", id: "new-api-token-value", value: token)
            )
        )
    })
}

#[component]
pub(super) async fn tokens_card(
    cx: &Cx,
    rows: &[TokenRow],
    form: &DialogForm,
    drafts: &TokenDrafts,
) -> Result<impl View> {
    Ok(view! {
        card(
            id: Some(AccountSection::ApiTokens.anchor()),
            card_header(
                title: "API tokens",
                create_token_button(form: form, drafts: drafts)
            )
            card_body(
                <p>
                    "API tokens let your own scripts and apps use your account. Treat them like passwords: anyone with a token can do whatever it allows. "
                    <a href=(API_DOCS_PATH)>"Read the API reference"</a>
                    "."
                </p>
                if rows.is_empty() {
                    empty_state(
                        heading: "No API tokens yet",
                        <p>"Create one to use the API from your own code."</p>
                    )
                } else {
                    property_list(
                        #[key(row.id.as_str())]
                        for row in rows {
                            token_item(row: row)
                        }
                    )
                }
            )
        )
        #[key(row.id.as_str())]
        for row in rows {
            revoke_token_dialog(row: row)
        }
    })
}

#[component]
async fn create_token_button(
    cx: &Cx,
    form: &DialogForm,
    drafts: &TokenDrafts,
) -> Result<impl View> {
    let DialogForm {
        error_field,
        error_message,
        ..
    } = form;
    let TokenDrafts {
        blank,
        name,
        password,
        created,
        ..
    } = drafts;

    Ok(view! {
        button(
            variant: ButtonVariant::Secondary,
            size: ButtonSize::Sm,
            attrs: attributes! {
                command="show-modal"
                commandfor=(CREATE_MODAL)
                @click=$(|_e: topcoat::runtime::Event| {
                    blank.set("".to_owned());
                    name.set("".to_owned());
                    password.set("".to_owned());
                    error_field.set("".to_owned());
                    error_message.set("".to_owned());
                    created.set("".to_owned());
                })
            },
            icon(name: IconName::Plus)
            "Create API token"
        )
    })
}

#[component]
async fn token_item(cx: &Cx, row: &TokenRow) -> Result<impl View> {
    let revoke_modal = format!("{REVOKE_MODAL}-{}", row.id);
    let revoke_label = format!("Revoke {}", row.name);

    Ok(view! {
        property(
            name: row.name.as_str(),
            tags: &row.tags,
            detail: Some(row.detail.as_str()),
            button(
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                attrs: attributes! {
                    command="show-modal"
                    commandfor=(revoke_modal.as_str())
                    aria-label=(revoke_label)
                },
                "Revoke"
            )
        )
    })
}

#[component]
async fn revoke_token_dialog(cx: &Cx, row: &TokenRow) -> Result<impl View> {
    let revoke_modal = format!("{REVOKE_MODAL}-{}", row.id);
    let revoke_action = href!(revoke_api_token_submit).resolve(cx);

    Ok(view! {
        confirm_dialog(
            id: &revoke_modal,
            heading: "Revoke API token",
            confirm_label: "Revoke token",
            action: Some(&revoke_action),
            <input type="hidden" name=(ID_FIELD) value=(row.id.as_str())>
            <p>
                "Revoke "
                <strong>(row.name.as_str())</strong>
                "? Anything using it will stop working right away."
            </p>
        )
    })
}
