use crate::action::{Caller, Channel};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventChannel {
    Browser,
    ApiToken(i32),
    App,
    Admin,
    Console,
    Stripe,
}

impl EventChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Browser => "browser",
            Self::ApiToken(_) => "api_token",
            Self::App => "app",
            Self::Admin => "admin",
            Self::Console => "console",
            Self::Stripe => "stripe",
        }
    }

    pub fn api_token_id(self) -> Option<i32> {
        match self {
            Self::ApiToken(id) => Some(id),
            _ => None,
        }
    }

    pub fn parse(channel: &str) -> Self {
        match channel {
            "api_token" => Self::ApiToken(0),
            "app" => Self::App,
            "admin" => Self::Admin,
            "console" => Self::Console,
            "stripe" => Self::Stripe,
            _ => Self::Browser,
        }
    }
}

pub struct EventSource {
    pub user_id: i32,
    pub channel: EventChannel,
    pub app: Option<String>,
}

impl EventSource {
    pub fn of(caller: &Caller) -> Self {
        let channel = match caller.via {
            Channel::Browser => EventChannel::Browser,
            Channel::ApiToken { id, .. } => EventChannel::ApiToken(id),
            Channel::App(_) => EventChannel::App,
        };
        Self {
            user_id: caller.user.id,
            channel,
            app: caller.app().map(|app| app.label.clone()),
        }
    }

    pub fn browser(user_id: i32) -> Self {
        Self {
            user_id,
            channel: EventChannel::Browser,
            app: None,
        }
    }

    pub fn admin(user_id: i32) -> Self {
        Self {
            user_id,
            channel: EventChannel::Admin,
            app: None,
        }
    }

    pub fn stripe(user_id: i32) -> Self {
        Self {
            user_id,
            channel: EventChannel::Stripe,
            app: None,
        }
    }

    pub fn console(user_id: i32) -> Self {
        Self {
            user_id,
            channel: EventChannel::Console,
            app: None,
        }
    }
}
