use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, view},
};

use super::styleguide_section;
use crate::components::button::{ButtonType, button};
use crate::components::copy_field::copy_field;
use crate::components::form::{
    Choice, InputType, LabelLink, checkbox, checkbox_group, code_field, form, form_actions,
    form_col, form_error, form_field, form_row, radio_group, select_field, textarea_field,
};

const PLANS: &[Choice<'static>] = &[
    Choice {
        value: "starter",
        label: "Starter",
    },
    Choice {
        value: "team",
        label: "Team",
    },
    Choice {
        value: "enterprise",
        label: "Enterprise",
    },
];

const BILLING_PERIODS: &[Choice<'static>] = &[
    Choice {
        value: "monthly",
        label: "Monthly",
    },
    Choice {
        value: "yearly",
        label: "Yearly (save 20%)",
    },
];

const NOTIFICATIONS: &[Choice<'static>] = &[
    Choice {
        value: "product",
        label: "Product updates",
    },
    Choice {
        value: "security",
        label: "Security alerts",
    },
    Choice {
        value: "billing",
        label: "Billing reminders",
    },
];

#[component]
pub async fn example_form_fields(cx: &Cx, id_prefix: &str) -> Result<impl View> {
    let first_name_id = format!("{id_prefix}-first-name");
    let email_id = format!("{id_prefix}-email");
    let password_id = format!("{id_prefix}-password");
    let confirm_password_id = format!("{id_prefix}-confirm-password");

    Ok(view! {
        form_field(
            label: "First name",
            id: &first_name_id,
            name: "first_name",
            placeholder: Some("Andrew")
        )
        form_field(
            label: "Email",
            id: &email_id,
            name: "email",
            input_type: InputType::Email,
            placeholder: Some("hello@andrewrobbins.dev")
        )
        form_row(
            form_col(
                half: true,
                form_field(
                    label: "New password",
                    id: &password_id,
                    name: "password",
                    input_type: InputType::Password,
                    placeholder: Some("Enter a password"),
                    required: true
                )
            )
            form_col(
                half: true,
                form_field(
                    label: "Confirm password",
                    id: &confirm_password_id,
                    name: "confirm_password",
                    input_type: InputType::Password,
                    placeholder: Some("Confirm your password"),
                    required: true
                )
            )
        )
    })
}

#[component]
pub async fn forms_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Forms",
            form(
                example_form_fields(id_prefix: "styleguide-form")
                select_field(
                    label: "Plan",
                    id: "styleguide-plan",
                    name: "plan",
                    options: PLANS,
                    placeholder: Some("Choose a plan")
                )
                radio_group(
                    legend: "Billing period",
                    id: "styleguide-billing-period",
                    name: "billing_period",
                    options: BILLING_PERIODS,
                    selected: Some("monthly")
                )
                checkbox_group(
                    legend: "Email me about",
                    id: "styleguide-notifications",
                    name: "notifications",
                    options: NOTIFICATIONS,
                    checked: &["security"]
                )
                textarea_field(
                    label: "Message",
                    id: "styleguide-message",
                    name: "message",
                    placeholder: Some("Tell us what you're building")
                )
                checkbox(
                    label: "I agree to the terms of service",
                    id: "styleguide-terms",
                    name: "terms"
                )
                form_actions(button(button_type: ButtonType::Submit, "Submit"))
            )
            <h3 class="styleguide__subheading">"Error state"</h3>
            form(
                form_field(
                    label: "Work email",
                    id: "styleguide-work-email",
                    name: "work_email",
                    input_type: InputType::Email,
                    value: Some("not-an-email"),
                    invalid: true
                )
                select_field(
                    label: "Tier",
                    id: "styleguide-invalid-plan",
                    name: "plan",
                    options: PLANS,
                    placeholder: Some("Choose a plan"),
                    invalid: true
                )
                checkbox_group(
                    legend: "Notify me about",
                    id: "styleguide-invalid-notifications",
                    name: "notifications",
                    options: NOTIFICATIONS,
                    invalid: true
                )
                textarea_field(
                    label: "Project description",
                    id: "styleguide-invalid-message",
                    name: "message",
                    value: Some("Too short"),
                    invalid: true
                )
                form_error("Please fix the highlighted fields.")
                form_actions(button(button_type: ButtonType::Submit, "Submit"))
            )
            <h3 class="styleguide__subheading">"Disabled"</h3>
            form(
                form_field(
                    label: "Email",
                    id: "styleguide-disabled-email",
                    name: "email",
                    value: Some("hello@andrewrobbins.dev"),
                    attrs: attributes! { disabled=(true) }
                )
            )
            <h3 class="styleguide__subheading">"Label link"</h3>
            form(
                form_field(
                    label: "Password",
                    id: "styleguide-label-link-password",
                    name: "password",
                    input_type: InputType::Password,
                    placeholder: Some("Enter password"),
                    label_link: Some(
                        LabelLink {
                            href: "#styleguide-label-link-password",
                            text: "Forgot password?",
                        },
                    )
                )
            )
            <h3 class="styleguide__subheading">"One-time code"</h3>
            form(
                code_field(
                    label: "Authentication code",
                    id: "styleguide-code",
                    name: "code"
                )
            )
            <h3 class="styleguide__subheading">"Copy field"</h3>
            form(
                copy_field(
                    label: "API token",
                    id: "styleguide-token",
                    value: "wsk_live_4f9a2c7e1b8d3f60"
                )
            )
        )
    })
}
