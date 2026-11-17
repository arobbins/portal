use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::activity::pages::activity_card::activity_card;
use crate::devices::pages::devices_card::devices_card;
use crate::flash::{Flash, flash_toast, flash_toasts, take_flash};

const FLASHES: &[Flash] = &[Flash::DeviceSignedOut, Flash::OtherDevicesSignedOut];

#[component]
pub async fn devices_page(cx: &Cx) -> Result<impl View> {
    let toast = flash_toast(cx, take_flash(cx, FLASHES).await?);

    Ok(view! {
        <div class="p-account__sub-page-content">
            devices_card()
            activity_card()
            flash_toasts(toast: &toast, flashes: FLASHES)
        </div>
    })
}
