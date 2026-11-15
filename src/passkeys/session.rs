use std::future::Future;

use time::OffsetDateTime;
use topcoat::Result;
use tower_sessions::Session;
use webauthn_rs::prelude::{DiscoverableAuthentication, PasskeyRegistration};

use crate::auth::session::{passkey_prompt_granted, revoke_passkey_prompt};
use crate::passkeys::webauthn::{
    AUTHENTICATION_STATE, REGISTRATION_STATE, store_ceremony, take_ceremony,
};
use crate::user::model::User;

pub trait PasskeySession: Sync {
    fn store_registration(
        &self,
        state: PasskeyRegistration,
        now: OffsetDateTime,
    ) -> impl Future<Output = Result<()>> + Send;

    fn take_registration(
        &self,
        now: OffsetDateTime,
    ) -> impl Future<Output = Result<Option<PasskeyRegistration>>> + Send;

    fn store_sign_in(
        &self,
        state: DiscoverableAuthentication,
        now: OffsetDateTime,
    ) -> impl Future<Output = Result<()>> + Send;

    fn take_sign_in(
        &self,
        now: OffsetDateTime,
    ) -> impl Future<Output = Result<Option<DiscoverableAuthentication>>> + Send;

    fn prompt_granted(
        &self,
        user: &User,
        now: OffsetDateTime,
    ) -> impl Future<Output = Result<bool>> + Send;

    fn revoke_prompt(&self) -> impl Future<Output = Result<()>> + Send;
}

impl PasskeySession for Session {
    async fn store_registration(
        &self,
        state: PasskeyRegistration,
        now: OffsetDateTime,
    ) -> Result<()> {
        store_ceremony(self, REGISTRATION_STATE, state, now).await
    }

    async fn take_registration(&self, now: OffsetDateTime) -> Result<Option<PasskeyRegistration>> {
        take_ceremony(self, REGISTRATION_STATE, now).await
    }

    async fn store_sign_in(
        &self,
        state: DiscoverableAuthentication,
        now: OffsetDateTime,
    ) -> Result<()> {
        store_ceremony(self, AUTHENTICATION_STATE, state, now).await
    }

    async fn take_sign_in(
        &self,
        now: OffsetDateTime,
    ) -> Result<Option<DiscoverableAuthentication>> {
        take_ceremony(self, AUTHENTICATION_STATE, now).await
    }

    async fn prompt_granted(&self, user: &User, now: OffsetDateTime) -> Result<bool> {
        passkey_prompt_granted(self, user, now).await
    }

    async fn revoke_prompt(&self) -> Result<()> {
        revoke_passkey_prompt(self).await
    }
}
