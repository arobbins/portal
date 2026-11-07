use axum::http::{HeaderMap, header::COOKIE};

pub const THEME_COOKIE: &str = "theme";
const ONE_YEAR_SECONDS: u32 = 60 * 60 * 24 * 365;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    Light,
    #[default]
    Dark,
    System,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Light, Theme::Dark, Theme::System];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::System => "System",
        }
    }

    pub fn color_scheme(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "light dark",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|theme| theme.as_str() == slug)
    }

    pub fn from_headers(headers: &HeaderMap) -> Self {
        headers
            .get_all(COOKIE)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == THEME_COOKIE)
            .and_then(|(_, value)| Self::from_slug(value))
            .unwrap_or_default()
    }

    pub fn set_cookie(self) -> String {
        format!(
            "{THEME_COOKIE}={}; Path=/; Max-Age={ONE_YEAR_SECONDS}; SameSite=Lax",
            self.as_str()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with_cookie(cookie: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(COOKIE, HeaderValue::from_str(cookie).unwrap());
        headers
    }

    #[test]
    fn defaults_to_dark_without_cookie() {
        assert_eq!(Theme::from_headers(&HeaderMap::new()), Theme::Dark);
    }

    #[test]
    fn reads_theme_among_other_cookies() {
        let headers = headers_with_cookie("id=abc; theme=light; other=1");
        assert_eq!(Theme::from_headers(&headers), Theme::Light);
    }

    #[test]
    fn ignores_unknown_values() {
        let headers = headers_with_cookie("theme=purple");
        assert_eq!(Theme::from_headers(&headers), Theme::Dark);
    }
}
