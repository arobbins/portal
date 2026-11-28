use std::convert::Infallible;

use crate::action::{ActionResult, Caller};
use crate::api::action::{ApiAction, NoInput};
use crate::api::catalog::HttpMethod;
use crate::env::Env;
use crate::permission::Permission;
use crate::profile::actions::{ProfileView, UpdateProfile, get_profile, update_profile};
use crate::user::name::NameProblem;

pub struct GetProfileAction;

impl ApiAction for GetProfileAction {
    const ID: &'static str = "get-profile";
    const GROUP: &'static str = "Profile";
    const METHOD: HttpMethod = HttpMethod::Get;
    const PATH: &'static str = "/api/v1/me";
    const SUMMARY: &'static str = "Get your profile";
    const DESCRIPTION: &'static str =
        "Returns the name and email of the account the token belongs to.";
    const PERMISSION: Permission = Permission::ViewProfile;

    type Input = NoInput;
    type Output = ProfileView;
    type Problem = Infallible;

    async fn run(
        _env: &Env<'_>,
        caller: &Caller,
        _input: &NoInput,
    ) -> ActionResult<ProfileView, Infallible> {
        Ok(get_profile(caller))
    }

    fn example_input() -> Option<NoInput> {
        None
    }

    fn example_output() -> ProfileView {
        sample_profile()
    }
}

pub struct UpdateProfileAction;

impl ApiAction for UpdateProfileAction {
    const ID: &'static str = "update-profile";
    const GROUP: &'static str = "Profile";
    const METHOD: HttpMethod = HttpMethod::Patch;
    const PATH: &'static str = "/api/v1/me";
    const SUMMARY: &'static str = "Update your profile";
    const DESCRIPTION: &'static str = "Changes the account's display name and returns the updated profile. The change appears in the app straight away and is recorded in the account's activity log.";
    const PERMISSION: Permission = Permission::EditProfile;

    type Input = UpdateProfile;
    type Output = ProfileView;
    type Problem = NameProblem;

    async fn run(
        env: &Env<'_>,
        caller: &Caller,
        input: &UpdateProfile,
    ) -> ActionResult<ProfileView, NameProblem> {
        update_profile(env, caller, input).await
    }

    fn example_input() -> Option<UpdateProfile> {
        Some(UpdateProfile {
            name: "Ada Lovelace".to_string(),
        })
    }

    fn example_output() -> ProfileView {
        sample_profile()
    }
}

fn sample_profile() -> ProfileView {
    ProfileView {
        name: "Ada Lovelace".to_string(),
        email: "ada@example.com".to_string(),
    }
}
