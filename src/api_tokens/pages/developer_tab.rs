use crate::routes::api_docs_url;
use topcoat::{
    Result,
    context::{Cx, try_context},
    router::{
        Body, Method,
        content::Form,
        error::{SeeOther, not_found, rewrite, see_other},
        href, route,
    },
    runtime::{Event, procedure, signal},
    view::{View, attributes, component, view},
};

use crate::account::tabs::{AccountSection, AccountTab};
use crate::action::{ActionProblem, FieldError, ID_FIELD, browser_caller, require_browser_caller};
use crate::api::scope::{ApiScope, TokenAudience, parse_scopes};
use crate::api_tokens::actions::{
    CreateToken, CreateTokenProblem, RevokeToken, TokenLifetime, create_api_token, list_api_tokens,
    revoke_api_token,
};
use crate::api_tokens::queries::ApiTokenSummary;
use crate::api_tokens::token::display_hint;
use crate::auth::password::CURRENT_PASSWORD_FIELD;
use crate::components::button::{ButtonSize, ButtonType, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::copy_field::copy_field;
use crate::components::empty_state::empty_state;
use crate::components::form::{
    Autocomplete, Choice, FORM_ERROR_ID, InputType, form, form_actions, form_error, form_field,
    select_field,
};
use crate::components::icon::{IconName, icon};
use crate::components::modal::modal;
use crate::components::notice::notice;
use crate::components::property_list::{PropertyTag, property, property_list};
use crate::components::toaster::{NO_TOAST, toaster};
use crate::components::tone::Tone;
use crate::flash::{Flash, flash_toasts, set_flash, take_flash, toast_name};
use crate::layout::page_heading::{PageHeadingVariant, page_heading};
use crate::user::name::{MAX_NAME_LENGTH, NAME_FIELD};
use crate::utils::format_date;
use crate::web::session;

const CREATE_MODAL: &str = "create-api-token";
const REVOKE_MODAL: &str = "revoke-api-token";
const FLASHES: &[Flash] = &[Flash::ApiTokenRevoked];
const ERROR_TOAST: &str = "error";
const STEP_CLASS: &str = "p-account__api-token-step";
const HIDDEN_STEP_CLASS: &str = "p-account__api-token-step p-account__api-token-step--is-hidden";
const REVEAL_CLASS: &str = "p-account__api-token-step p-account__api-token-reveal";
const HIDDEN_REVEAL_CLASS: &str =
    "p-account__api-token-step p-account__api-token-reveal p-account__api-token-step--is-hidden";
const DEFAULT_ACCESS: ApiScope = ApiScope::ProfileRead;
const UNREACHABLE: &str = "We couldn't create the token. Please try again.";

struct CreateTokenFailed {
    problem: CreateTokenProblem,
}

struct TokenCreated {
    token: String,
}

struct TokenRow {
    id: String,
    name: String,
    tags: Vec<PropertyTag>,
    detail: String,
}

impl TokenRow {
    fn from_summary(token: &ApiTokenSummary) -> Self {
        let now = time::OffsetDateTime::now_utc();
        let expired = token.expires_at <= now;
        let mut tags: Vec<PropertyTag> = parse_scopes(&token.scopes)
            .into_iter()
            .map(|scope| PropertyTag::new(scope.as_str(), Tone::Neutral))
            .collect();
        if TokenAudience::from_str(&token.audience) == Some(TokenAudience::Mcp) {
            tags.push(PropertyTag::new("MCP", Tone::Info));
        }
        if expired {
            tags.push(PropertyTag::new("Expired", Tone::Error));
        }
        let expiry = if expired {
            format!("expired {}", format_date(token.expires_at))
        } else {
            format!("expires {}", format_date(token.expires_at))
        };
        let last_used = token.last_used_at.map_or_else(
            || "never used".to_string(),
            |at| format!("last used {}", format_date(at)),
        );

        Self {
            id: token.public_id.clone(),
            name: token.name.clone(),
            tags,
            detail: format!(
                "{} · Created {} · {expiry} · {last_used}",
                display_hint(&token.last_four),
                format_date(token.created_at)
            ),
        }
    }
}

#[component]
pub async fn developer_tab(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let flash = take_flash(cx, FLASHES).await?;
    let failed = try_context::<CreateTokenFailed>(cx).map(|failed| failed.problem);
    let created = try_context::<TokenCreated>(cx).map(|created| created.token.clone());
    let tokens = match list_api_tokens(cx, &caller).await? {
        Ok(tokens) => tokens,
        Err(error) => match error.for_browser()? {},
    };
    let rows: Vec<TokenRow> = tokens.iter().map(TokenRow::from_summary).collect();

    let blank = signal(cx, String::new);
    let name_draft = signal(cx, String::new);
    let access_draft = signal(cx, || DEFAULT_ACCESS.as_str().to_string());
    let lifetime_draft = signal(cx, || TokenLifetime::DEFAULT.value().to_string());
    let audience_draft = signal(cx, || TokenAudience::DEFAULT.as_str().to_string());
    let password_draft = signal(cx, String::new);
    let error_field = signal(cx, String::new);
    let error_message = signal(cx, || failed.map_or("", ActionProblem::message).to_string());
    let new_token = signal(cx, String::new);
    let is_submitting = signal(cx, || false);
    let initial_toast = if failed.is_some() {
        ERROR_TOAST
    } else {
        toast_name(flash)
    };
    let toast = signal(cx, || initial_toast.to_string());

    let access_choices: Vec<Choice<'static>> = ApiScope::ALL
        .into_iter()
        .map(|scope| Choice {
            value: scope.as_str(),
            label: scope.choice_label(),
        })
        .collect();
    let audience_choices: Vec<Choice<'static>> = TokenAudience::ALL
        .into_iter()
        .map(|audience| Choice {
            value: audience.as_str(),
            label: audience.label(),
        })
        .collect();
    let lifetime_choices: Vec<Choice<'static>> = TokenLifetime::ALL
        .into_iter()
        .map(|lifetime| Choice {
            value: lifetime.value(),
            label: lifetime.label(),
        })
        .collect();
    let name_field = NAME_FIELD.to_string();
    let password_field = CURRENT_PASSWORD_FIELD.to_string();
    let create_failed: std::result::Result<String, FieldError> =
        Err(FieldError::general(UNREACHABLE));
    let max_name_length = MAX_NAME_LENGTH.to_string();
    let developer_url = AccountTab::Developer.href(cx);
    let docs_url = api_docs_url(cx);

    Ok(view! {
        <div class="p-account__sub-page-content">
            page_heading(text: "Developer", variant: PageHeadingVariant::Compact)
            if let Some(token) = &created {
                card(
                    id: Some("new-api-token"),
                    card_header(title: "Your new API token")
                    card_body(
                        notice(
                            tone: Tone::Warning,
                            "Copy this token now. You won't be able to see it again."
                        )
                        copy_field(
                            label: "API token",
                            id: "new-api-token-value",
                            value: token
                        )
                    )
                )
            }
            card(
                id: Some(AccountSection::ApiTokens.anchor()),
                card_header(
                    title: "API tokens",
                    button(
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        attrs: attributes! {
                            command="show-modal"
                            commandfor=(CREATE_MODAL)
                            @click=$(|_e: Event| {
                                blank.set("".to_owned());
                                name_draft.set("".to_owned());
                                password_draft.set("".to_owned());
                                error_field.set("".to_owned());
                                error_message.set("".to_owned());
                                new_token.set("".to_owned());
                            })
                        },
                        icon(name: IconName::Plus)
                        "Create API token"
                    )
                )
                card_body(
                    <p>
                        "API tokens let your own scripts and apps use your account. Treat them like passwords: anyone with a token can do whatever it allows. "
                        <a href=(docs_url.as_str())>"Read the API reference"</a>
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
                            for row in &rows {
                                api_token_item(row: row)
                            }
                        )
                    }
                )
            )
            modal(
                id: CREATE_MODAL,
                heading: "Create API token",
                <div
                    :class=$(if new_token.get().is_empty() {
                        STEP_CLASS
                    } else {
                        HIDDEN_STEP_CLASS
                    })
                >
                    form(
                        attrs: attributes! {
                            method="post"
                            action=(href!(create_api_token_submit))
                            @submit=$(async |e: Event| {
                                e.prevent_default();
                                is_submitting.set(true);
                                error_field.set("".to_owned());
                                error_message.set("".to_owned());
                                toast.set(NO_TOAST.to_owned());
                                let _pending = create_api_token_js(
                                    name_draft.get(),
                                    access_draft.get(),
                                    lifetime_draft.get(),
                                    audience_draft.get(),
                                    password_draft.get(),
                                );
                                let result = raw!(
                                    "await Promise.resolve(${_pending}).catch(() => ${create_failed})",
                                    create_failed.clone(),
                                );
                                is_submitting.set(false);
                                if result.is_ok() {
                                    blank.set("".to_owned());
                                    password_draft.set("".to_owned());
                                    new_token.set(result.unwrap());
                                } else {
                                    let failure = result.unwrap_err();
                                    error_field.set(failure.field);
                                    error_message.set(failure.message);
                                }
                            })
                        },
                        form_field(
                            label: "Name",
                            id: "api-token-name",
                            name: NAME_FIELD,
                            required: true,
                            placeholder: Some("e.g. Deploy script"),
                            attrs: attributes! {
                                maxlength=(max_name_length.as_str())
                                :value=$(blank.get())
                                :aria-invalid=$(if error_field.get() == name_field {
                                    "true"
                                } else {
                                    "false"
                                })
                                aria-describedby=(FORM_ERROR_ID)
                                @input=$(|e: Event| name_draft.set(e.target.value))
                            }
                        )
                        select_field(
                            label: "Access",
                            id: "api-token-access",
                            name: "access",
                            options: &access_choices,
                            selected: Some(DEFAULT_ACCESS.as_str()),
                            required: true,
                            attrs: attributes! { @change=$(|e: Event| access_draft.set(e.target.value)) }
                        )
                        select_field(
                            label: "Used by",
                            id: "api-token-audience",
                            name: "audience",
                            options: &audience_choices,
                            selected: Some(TokenAudience::DEFAULT.as_str()),
                            required: true,
                            attrs: attributes! { @change=$(|e: Event| audience_draft.set(e.target.value)) }
                        )
                        select_field(
                            label: "Expires after",
                            id: "api-token-lifetime",
                            name: "lifetime",
                            options: &lifetime_choices,
                            selected: Some(TokenLifetime::DEFAULT.value()),
                            required: true,
                            attrs: attributes! { @change=$(|e: Event| lifetime_draft.set(e.target.value)) }
                        )
                        form_field(
                            label: "Current password",
                            id: "api-token-password",
                            autocomplete: Some(Autocomplete::CurrentPassword),
                            name: CURRENT_PASSWORD_FIELD,
                            input_type: InputType::Password,
                            required: true,
                            attrs: attributes! {
                                :value=$(blank.get())
                                :aria-invalid=$(if error_field.get() == password_field {
                                    "true"
                                } else {
                                    "false"
                                })
                                aria-describedby=(FORM_ERROR_ID)
                                @input=$(|e: Event| password_draft.set(e.target.value))
                            }
                        )
                        form_error(
                            hidden: $(error_message.get().is_empty()),
                            $(error_message.get())
                        )
                        form_actions(
                            button(
                                button_type: ButtonType::Submit,
                                busy: $(is_submitting.get()),
                                "Create API token"
                            )
                        )
                    )
                </div>
                <div
                    :class=$(if new_token.get().is_empty() {
                        HIDDEN_REVEAL_CLASS
                    } else {
                        REVEAL_CLASS
                    })
                >
                    notice(
                        tone: Tone::Warning,
                        "Copy this token now. You won't be able to see it again."
                    )
                    copy_field(
                        label: "API token",
                        id: "api-token-value",
                        source: Some(&new_token)
                    )
                    form(
                        attrs: attributes! { method="get" action=(developer_url.as_str()) },
                        form_actions(button(button_type: ButtonType::Submit, "Done"))
                    )
                </div>
            )
            #[key(row.id.as_str())]
            for row in &rows {
                api_token_dialog(row: row)
            }
            flash_toasts(toast: &toast, flashes: FLASHES)
            toaster(
                toast: &toast,
                name: ERROR_TOAST,
                tone: Tone::Error,
                $(error_message.get())
            )
        </div>
    })
}

#[component]
async fn api_token_item(cx: &Cx, row: &TokenRow) -> Result<impl View> {
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
async fn api_token_dialog(cx: &Cx, row: &TokenRow) -> Result<impl View> {
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

#[procedure]
async fn create_api_token_js(
    cx: &Cx,
    name: String,
    access: String,
    lifetime: String,
    audience: String,
    current_password: String,
) -> Result<Result<String, FieldError>> {
    let Some(caller) = browser_caller(cx) else {
        return Ok(Err(FieldError::session_expired()));
    };
    let (Ok(access), Ok(lifetime), Some(audience)) = (
        serde_json::from_value::<ApiScope>(access.into()),
        serde_json::from_value::<TokenLifetime>(lifetime.into()),
        TokenAudience::from_str(&audience),
    ) else {
        return Ok(Err(FieldError::of(CreateTokenProblem::UnknownAccess)));
    };
    let input = CreateToken {
        name,
        access,
        lifetime,
        audience,
        current_password,
    };

    match create_api_token(cx, &caller, &input).await? {
        Ok(token) => Ok(Ok(token)),
        Err(error) => Ok(Err(FieldError::of(error.for_browser()?))),
    }
}

#[route(POST "/account/api-tokens/create")]
pub async fn create_api_token_submit(cx: &Cx, Form(input): Form<CreateToken>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;
    let back_to_page =
        || rewrite(AccountTab::Developer.href(cx), Body::empty()).method(Method::GET);

    match create_api_token(cx, &caller, &input).await? {
        Ok(token) => Err(back_to_page().with(TokenCreated { token }).into()),
        Err(error) => Err(back_to_page()
            .with(CreateTokenFailed {
                problem: error.for_browser()?,
            })
            .into()),
    }
}

#[route(POST "/account/api-tokens/revoke")]
pub async fn revoke_api_token_submit(cx: &Cx, Form(input): Form<RevokeToken>) -> Result<SeeOther> {
    let caller = require_browser_caller(cx)?;

    if let Err(error) = revoke_api_token(cx, &caller, &input).await? {
        error.for_browser()?;
        return Err(not_found().into());
    }
    set_flash(&session(cx)?, Flash::ApiTokenRevoked).await?;

    Ok(see_other(AccountSection::ApiTokens.href(cx)))
}
