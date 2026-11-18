use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::{special_fields, styleguide_section};
use crate::components::button::{ButtonType, ButtonVariant, button};
use crate::components::form::{
    Choice, InputType, checkbox, checkbox_group, form, form_actions, form_col, form_error,
    form_field, form_row, radio_group, select_field, textarea_field,
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
            all_controls_demo()
            <h3 class="p-styleguide__subheading">"Error state"</h3>
            error_state_demo()
            special_fields::special_fields_demo()
        )
    })
}

#[component]
async fn all_controls_demo(cx: &Cx) -> Result<impl View> {
    Ok(view! {
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
            form_actions(
                button(button_type: ButtonType::Submit, "Submit")
                button(
                    variant: ButtonVariant::Secondary,
                    button_type: ButtonType::Reset,
                    "Reset"
                )
            )
        )
    })
}

#[component]
async fn error_state_demo(cx: &Cx) -> Result<impl View> {
    Ok(view! {
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
    })
}
