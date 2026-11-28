use time::{Duration, OffsetDateTime};
use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, attributes, component, view},
};

use super::device_routes::{sign_out_device_submit, sign_out_other_devices_submit};
use crate::account::tabs::AccountSection;
use crate::action::{ForPage, ID_FIELD, require_browser_caller};
use crate::auth::session::current_device;
use crate::components::button::{ButtonSize, ButtonVariant, button};
use crate::components::card::{card, card_body, card_header};
use crate::components::confirm_dialog::confirm_dialog;
use crate::components::property_list::{PropertyTag, property, property_list};
use crate::components::tone::Tone;
use crate::devices::actions::{DeviceView, list_devices};
use crate::time_zone::Zone;
use crate::web::{app, env, session};

const OTHERS_MODAL: &str = "sign-out-other-devices";
const DEVICE_MODAL: &str = "sign-out-device";
const ACTIVE_NOW: Duration = Duration::minutes(10);

struct DeviceRow {
    id: String,
    name: String,
    current: bool,
    summary: String,
    modal: String,
}

impl DeviceRow {
    fn of(device: &DeviceView, now: OffsetDateTime, zone: &Zone) -> Self {
        let active = if device.current || now - device.last_active_at < ACTIVE_NOW {
            "Active now".to_string()
        } else {
            format!("Last active {}", zone.date(device.last_active_at))
        };
        let mut parts = vec![
            format!("Signed in {}", zone.date(device.signed_in_at)),
            active,
        ];
        parts.extend(device.ip.clone());

        Self {
            modal: format!("{DEVICE_MODAL}-{}", device.id),
            id: device.id.clone(),
            name: device.name.clone(),
            current: device.current,
            summary: parts.join(" · "),
        }
    }
}

#[component]
pub async fn devices_card(cx: &Cx) -> Result<impl View> {
    let caller = require_browser_caller(cx)?;
    let current = current_device(&session(cx)?).await?;
    let devices = list_devices(&env(cx), &caller, current.as_deref())
        .await?
        .for_page()?;
    let now = app(cx).clock.now();
    let zone = caller.user.zone();
    let rows: Vec<DeviceRow> = devices
        .iter()
        .map(|device| DeviceRow::of(device, now, &zone))
        .collect();
    let has_others = rows.iter().any(|row| !row.current);
    let sign_out_action = href!(sign_out_device_submit).resolve(cx);
    let sign_out_others_action = href!(sign_out_other_devices_submit).resolve(cx);

    Ok(view! {
        card(
            id: Some(AccountSection::Devices.anchor()),
            card_header(
                title: "Devices",
                if has_others {
                    button(
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        attrs: attributes! { command="show-modal" commandfor=(OTHERS_MODAL) },
                        "Sign out other devices"
                    )
                }
            )
            card_body(
                <p>
                    "These browsers are signed in to your account. Sign out any you don't recognize."
                </p>
                property_list(
                    #[key(row.id.as_str())]
                    for row in &rows {
                        device_item(row: row)
                    }
                )
            )
        )
        #[key(row.id.as_str())]
        for row in rows.iter().filter(|row| !row.current) {
            confirm_dialog(
                id: &row.modal,
                heading: "Sign out device",
                confirm_label: "Sign out device",
                action: Some(&sign_out_action),
                <input type="hidden" name=(ID_FIELD) value=(row.id.as_str())>
                <p>
                    "Sign out "
                    <strong>(row.name.as_str())</strong>
                    "? Anyone using it will need to sign in again."
                </p>
            )
        }
        confirm_dialog(
            id: OTHERS_MODAL,
            heading: "Sign out other devices",
            confirm_label: "Sign out other devices",
            action: Some(&sign_out_others_action),
            <p>"Every browser except this one will be signed out of your account."</p>
        )
    })
}

#[component]
async fn device_item(cx: &Cx, row: &DeviceRow) -> Result<impl View> {
    let tags = if row.current {
        vec![PropertyTag::new("This device", Tone::Success)]
    } else {
        Vec::new()
    };
    let label = format!("Sign out {}", row.name);

    Ok(view! {
        property(
            name: row.name.as_str(),
            tags: &tags,
            detail: Some(row.summary.as_str()),
            if !row.current {
                button(
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    attrs: attributes! {
                        command="show-modal"
                        commandfor=(row.modal.as_str())
                        aria-label=(label)
                    },
                    "Sign out device"
                )
            }
        )
    })
}
