use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::{error::not_found, href, page, path_param},
    view::{View, component, view},
};

use super::admin_access::require_admin;
use super::user_actions::user_actions_card;
use super::user_routes::admin_action_problem;
use super::user_view::{listed_status, role_label, status_label};
use super::users_page::admin_users_page;
use crate::action::ActionProblem;
use crate::activity::pages::activity_card::activity_list;
use crate::admin::actions::{TargetUser, UserOverview, get_user};
use crate::app::page_title;
use crate::components::card::{card, card_body, card_header};
use crate::components::notice::notice;
use crate::components::property_list::{PropertyTag, property, property_list};
use crate::components::tone::Tone;
use crate::flash::{Flash, flash_toast, flash_toasts, take_flash};
use crate::layout::page_heading::page_heading;
use crate::web::env;

path_param!(pub id);

const FLASHES: &[Flash] = &[
    Flash::UserDisabled,
    Flash::UserEnabled,
    Flash::UserSignedOut,
    Flash::VerificationResent,
];

#[page("/admin/users/{id}", with(Metadata::new().title(page_title("User"))))]
pub async fn admin_user_page(cx: &Cx) -> Result<impl View> {
    let caller = require_admin(cx).await?;
    let target = TargetUser {
        id: id(cx).to_string(),
    };
    let Ok(overview) = get_user(&env(cx), &caller, &target).await? else {
        return Err(not_found().into());
    };
    let problem = admin_action_problem(cx);
    let toast = flash_toast(cx, take_flash(cx, FLASHES).await?);
    let is_self = overview.user.id == caller.user.id;
    let back = href!(admin_users_page).resolve(cx);

    Ok(view! {
        <section class="l-contain p-admin">
            <a class="p-admin__back" href=(back)>"All users"</a>
            page_heading(text: overview.user.name.as_str())
            if let Some(problem) = problem {
                notice(tone: Tone::Error, (problem.message()))
            }
            account_card(overview: &overview)
            user_actions_card(user: &overview.user, is_self: is_self)
            card(
                card_header(title: "Recent activity")
                card_body(activity_list(activity: &overview.activity))
            )
            flash_toasts(toast: &toast, flashes: FLASHES)
        </section>
    })
}

#[component]
async fn account_card(cx: &Cx, overview: &UserOverview) -> Result<impl View> {
    let user = &overview.user;
    let (status, tone) = status_label(listed_status(user));
    let status_tags = vec![PropertyTag::new(status, tone)];
    let counts = &overview.counts;
    let security = format!(
        "{} passkeys · {} signed-in devices · {} API tokens",
        counts.passkeys, counts.devices, counts.api_tokens
    );
    let joined = crate::utils::format_date(counts.created_at);

    Ok(view! {
        card(
            card_header(title: "Account")
            card_body(
                property_list(
                    property(name: "Email", description: Some(user.email.as_str()))
                    property(name: "Status", tags: &status_tags)
                    property(name: "Role", description: Some(role_label(user.role)))
                    property(name: "Joined", description: Some(joined.as_str()))
                    property(name: "Security", description: Some(security.as_str()))
                    property(
                        name: "User id",
                        description: Some(user.public_id.as_str())
                    )
                )
            )
        )
    })
}
