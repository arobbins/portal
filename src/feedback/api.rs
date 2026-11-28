use crate::action::{ActionResult, Caller};
use crate::api::action::ApiAction;
use crate::api::catalog::HttpMethod;
use crate::env::Env;
use crate::feedback::actions::{
    FeedbackCategory, FeedbackProblem, FeedbackReceipt, SendApiFeedback, send_api_feedback,
};
use crate::permission::Permission;

pub struct SendFeedbackAction;

impl ApiAction for SendFeedbackAction {
    const ID: &'static str = "send-feedback";
    const GROUP: &'static str = "Feedback";
    const METHOD: HttpMethod = HttpMethod::Post;
    const PATH: &'static str = "/api/v1/feedback";
    const SUMMARY: &'static str = "Send feedback about the API";
    const DESCRIPTION: &'static str = "Tells the people who run this API what got in your way: a confusing doc, an unexpected error, a wrong schema or a missing capability. They read every message and use it to fix the API and its docs. Anything that looks like a token is removed before it's stored, and the feedback is recorded in the account's activity log.";
    const PERMISSION: Permission = Permission::SendApiFeedback;

    type Input = SendApiFeedback;
    type Output = FeedbackReceipt;
    type Problem = FeedbackProblem;

    async fn run(
        env: &Env<'_>,
        caller: &Caller,
        input: &SendApiFeedback,
    ) -> ActionResult<FeedbackReceipt, FeedbackProblem> {
        send_api_feedback(env, caller, input).await
    }

    fn example_input() -> Option<SendApiFeedback> {
        Some(SendApiFeedback {
            category: FeedbackCategory::ConfusingDocs,
            endpoint: Some("update-profile".to_string()),
            problem_code: Some("name_too_long".to_string()),
            request_id: None,
            message: "The docs don't say whether the 100-character limit counts bytes or characters, so I couldn't tell how far to shorten the name.".to_string(),
        })
    }

    fn example_output() -> FeedbackReceipt {
        FeedbackReceipt {
            id: "fbk_4f9a2c7e1b8d3f60a2Kq".to_string(),
        }
    }
}
