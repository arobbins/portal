use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::time_zone_card::{ZoneDrafts, time_zone_card, time_zone_dialog};
use super::time_zone_routes::time_zone_problem;
use crate::action::ActionProblem;
use crate::components::form::{DialogForm, dialog_error_toast, page_toast};
use crate::flash::{Flash, flash_toasts, take_flash};
use crate::user::model::User;

const FLASHES: &[Flash] = &[Flash::TimeZoneSaved];

#[component]
pub async fn time_zone_page(cx: &Cx, user: &User) -> Result<impl View> {
    let flash = take_flash(cx, FLASHES).await?;
    let failure = time_zone_problem(cx).map(ActionProblem::spec);
    let form = DialogForm::new(cx, failure);
    let toast = page_toast(cx, failure.is_some(), flash);
    let zone = user.zone();
    let drafts = ZoneDrafts::new(cx, zone.name());

    Ok(view! {
        <div class="p-account__sub-page-content">
            time_zone_card(form: &form, drafts: &drafts)
            time_zone_dialog(
                form: &form,
                drafts: &drafts,
                current: zone.name(),
                toast: &toast
            )
            flash_toasts(toast: &toast, flashes: FLASHES)
            dialog_error_toast(failure: failure, toast: &toast)
        </div>
    })
}
