use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, not_found, rewrite, see_other},
        href, route,
    },
    runtime::{Event, Signal, signal},
    view::{View, attributes, component, view},
};

use crate::components::badge::{BadgeKind, badge};
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::form::{
    Autocomplete, InputType, form, form_actions, form_error, form_field,
};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::notice::{NoticeKind, notice};
use crate::flash::{Flash, set_flash};
use crate::user::auth::{current_user, require_user};
use crate::user::model::Passkey;
use crate::user::name::{NameProblem, check_name};
use crate::user::queries::{query_delete_passkey, query_list_passkeys, query_rename_passkey};
use crate::web::{db, session};

const SECURITY_URL: &str = "/account/security";
const PASSKEYS_URL: &str = "/account/security#passkeys";
const ADD_MODAL: &str = "add-passkey";
const RENAME_MODAL: &str = "rename-passkey";
const REMOVE_MODAL: &str = "remove-passkey";
pub const ADD_ERROR_ID: &str = "add-passkey-error";

pub struct PasskeyRenameFailed {
    pub problem: NameProblem,
}

struct PasskeyRow {
    id: String,
    name: String,
    synced: bool,
    summary: String,
}

fn format_date(date: time::OffsetDateTime) -> String {
    let month = date.month().to_string();
    format!("{} {}, {}", &month[..3], date.day(), date.year())
}

impl PasskeyRow {
    fn from_passkey(passkey: &Passkey) -> Self {
        let last_used = passkey.last_used_at.map_or_else(
            || "never used".to_string(),
            |at| format!("last used {}", format_date(at)),
        );

        Self {
            id: passkey.id.to_string(),
            name: passkey.name.clone(),
            synced: passkey.backed_up,
            summary: format!("Added {} · {last_used}", format_date(passkey.created_at)),
        }
    }
}

#[component]
pub async fn account_passkeys(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx)?;
    let passkeys = query_list_passkeys(user.id, db(cx)).await?;
    let rows: Vec<PasskeyRow> = passkeys.iter().map(PasskeyRow::from_passkey).collect();
    let only_one = passkeys.len() == 1;
    let needs_backup = only_one && !passkeys[0].backup_eligible;

    let selected_id = signal(cx, String::new);
    let selected_name = signal(cx, String::new);

    Ok(view! {
        card(
            id: Some("passkeys"),
            card_header(
                title: "Passkeys",
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! { command="show-modal" commandfor=(ADD_MODAL) },
                    icon(name: IconName::Plus)
                    "Add passkey"
                )
            )
            card_body(
                if rows.is_empty() {
                    <p>
                        "Sign in with your fingerprint, face or screen lock instead of typing your password. Passkeys can't be phished or reused on other sites."
                    </p>
                } else {
                    <ul class="account__passkeys">
                        #[key(row.id.as_str())]
                        for row in &rows {
                            passkey_item(
                                row: row,
                                selected_id: &selected_id,
                                selected_name: &selected_name
                            )
                        }
                    </ul>
                }
                if needs_backup {
                    notice(
                        kind: NoticeKind::Warning,
                        "This passkey only exists on one device. Add a second passkey so you can still sign in if you lose it."
                    )
                }
            )
        )
        modal(
            id: ADD_MODAL,
            heading: "Add a passkey",
            form(
                attrs: attributes! { data-passkey-register="" },
                <p>"Confirm your password, then follow your device's prompt."</p>
                form_field(
                    label: "Current password",
                    id: "add-passkey-password",
                    autocomplete: Some(Autocomplete::CurrentPassword),
                    name: "current_password",
                    input_type: InputType::Password,
                    required: true,
                    attrs: attributes! { aria-describedby=(ADD_ERROR_ID) }
                )
                form_error(id: Some(ADD_ERROR_ID), hidden: true, "")
                <noscript><p>"Adding a passkey needs JavaScript."</p></noscript>
                form_actions(button(button_type: ButtonType::Submit, "Continue"))
            )
        )
        modal(
            id: RENAME_MODAL,
            heading: "Rename passkey",
            form(
                attrs: attributes! { method="post" action=(href!(rename_passkey)) },
                <input type="hidden" name="id" :value=$(selected_id.get())>
                form_field(
                    label: "Name",
                    id: "passkey-name",
                    name: "name",
                    required: true,
                    attrs: attributes! { :value=$(selected_name.get()) maxlength="100" }
                )
                form_actions(button(button_type: ButtonType::Submit, "Save"))
            )
        )
        modal(
            id: REMOVE_MODAL,
            heading: "Remove passkey",
            form(
                attrs: attributes! { method="post" action=(href!(remove_passkey)) },
                <input type="hidden" name="id" :value=$(selected_id.get())>
                <p>
                    "Remove "
                    <strong>$(selected_name.get())</strong>
                    "? You won't be able to sign in with it anymore."
                </p>
                if only_one {
                    <p>
                        "This is your only passkey. You'll still be able to sign in with your password."
                    </p>
                }
                form_actions(button(button_type: ButtonType::Submit, "Remove passkey"))
            )
        )
    })
}

#[component]
async fn passkey_item(
    cx: &Cx,
    row: &PasskeyRow,
    selected_id: &Signal<String>,
    selected_name: &Signal<String>,
) -> Result<impl View> {
    let id = row.id.clone();
    let name = row.name.clone();
    let rename_label = format!("Rename {}", row.name);
    let remove_label = format!("Remove {}", row.name);

    Ok(view! {
        <li class="account__passkey">
            <div class="account__passkey-info">
                <span class="account__passkey-name">(row.name.as_str())</span>
                if row.synced {
                    badge("Synced")
                } else {
                    badge(kind: BadgeKind::Warning, "This device only")
                }
                <span class="account__passkey-meta">(row.summary.as_str())</span>
            </div>
            <div class="account__passkey-actions">
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        command="show-modal"
                        commandfor=(RENAME_MODAL)
                        aria-label=(rename_label)
                        @click=$(|_e: Event| {
                            selected_id.set(id.clone());
                            selected_name.set(name.clone());
                        })
                    },
                    "Rename"
                )
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        command="show-modal"
                        commandfor=(REMOVE_MODAL)
                        aria-label=(remove_label)
                        @click=$(|_e: Event| {
                            selected_id.set(id.clone());
                            selected_name.set(name.clone());
                        })
                    },
                    "Remove"
                )
            </div>
        </li>
    })
}

#[derive(Deserialize)]
pub struct RenameInput {
    id: i32,
    name: String,
}

#[route(POST "/account/passkeys/rename")]
pub async fn rename_passkey(cx: &Cx, Form(input): Form<RenameInput>) -> Result<SeeOther> {
    let Some(user) = current_user(cx) else {
        return Ok(see_other("/login"));
    };
    let name = match check_name(&input.name) {
        Ok(name) => name,
        Err(problem) => {
            return Err(rewrite(SECURITY_URL, Body::empty())
                .method(Method::GET)
                .with(PasskeyRenameFailed { problem })
                .into());
        }
    };

    if !query_rename_passkey(input.id, user.id, name, db(cx)).await? {
        return Err(not_found().into());
    }
    set_flash(&session(cx)?, Flash::PasskeyRenamed).await?;

    Ok(see_other(PASSKEYS_URL))
}

#[derive(Deserialize)]
pub struct RemoveInput {
    id: i32,
}

#[route(POST "/account/passkeys/remove")]
pub async fn remove_passkey(cx: &Cx, Form(input): Form<RemoveInput>) -> Result<SeeOther> {
    let Some(user) = current_user(cx) else {
        return Ok(see_other("/login"));
    };

    if !query_delete_passkey(input.id, user.id, db(cx)).await? {
        return Err(not_found().into());
    }
    set_flash(&session(cx)?, Flash::PasskeyRemoved).await?;

    Ok(see_other(PASSKEYS_URL))
}

pub fn rename_problem(cx: &Cx) -> Option<NameProblem> {
    try_context::<PasskeyRenameFailed>(cx).map(|failed| failed.problem)
}
