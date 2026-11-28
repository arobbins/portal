use crate::api::tool::{Tool, public_tool, tool};
use crate::billing::api::{GetBillingAction, ListPlansAction, StartCheckoutAction};
use crate::feedback::api::SendFeedbackAction;
use crate::preferences::api::{GetNewslettersAction, UpdateNewslettersAction};
use crate::profile::api::{GetProfileAction, UpdateProfileAction};

pub fn tools() -> Vec<Tool> {
    vec![
        tool::<GetProfileAction>(),
        tool::<UpdateProfileAction>(),
        tool::<GetNewslettersAction>(),
        tool::<UpdateNewslettersAction>(),
        tool::<SendFeedbackAction>(),
        public_tool::<ListPlansAction>(),
        tool::<GetBillingAction>(),
        tool::<StartCheckoutAction>(),
    ]
}

pub fn feedback_tool() -> Tool {
    tool::<SendFeedbackAction>()
}
