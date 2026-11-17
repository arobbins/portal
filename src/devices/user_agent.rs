const MAX_USER_AGENT_LENGTH: usize = 512;

const BROWSERS: [(&str, &str); 7] = [
    ("Edg/", "Edge"),
    ("OPR/", "Opera"),
    ("SamsungBrowser/", "Samsung Internet"),
    ("Firefox/", "Firefox"),
    ("FxiOS/", "Firefox"),
    ("CriOS/", "Chrome"),
    ("Chrome/", "Chrome"),
];

const SYSTEMS: [(&str, &str); 7] = [
    ("iPhone", "iOS"),
    ("iPad", "iPadOS"),
    ("Android", "Android"),
    ("CrOS", "ChromeOS"),
    ("Mac OS X", "macOS"),
    ("Windows", "Windows"),
    ("Linux", "Linux"),
];

pub fn clip_user_agent(user_agent: &str) -> String {
    user_agent.chars().take(MAX_USER_AGENT_LENGTH).collect()
}

pub fn describe_user_agent(user_agent: &str) -> String {
    let browser = browser(user_agent);
    match system(user_agent) {
        Some(system) => format!("{browser} on {system}"),
        None => browser.to_string(),
    }
}

fn browser(user_agent: &str) -> &'static str {
    let named = BROWSERS
        .iter()
        .find(|(marker, _)| user_agent.contains(marker))
        .map(|(_, name)| *name);
    match named {
        Some(name) => name,
        None if user_agent.contains("Safari/") => "Safari",
        None => "Unknown browser",
    }
}

fn system(user_agent: &str) -> Option<&'static str> {
    SYSTEMS
        .iter()
        .find(|(marker, _)| user_agent.contains(marker))
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_browsers_get_a_readable_name() {
        let cases = [
            (
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
                "Chrome on macOS",
            ),
            (
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0",
                "Edge on Windows",
            ),
            (
                "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0",
                "Firefox on Linux",
            ),
            (
                "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1",
                "Safari on iOS",
            ),
            (
                "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36",
                "Chrome on Android",
            ),
            ("curl/8.7.1", "Unknown browser"),
            ("", "Unknown browser"),
        ];

        for (user_agent, expected) in cases {
            assert_eq!(describe_user_agent(user_agent), expected, "{user_agent}");
        }
    }

    #[test]
    fn long_user_agents_are_clipped_to_the_column_limit() {
        let long = "é".repeat(MAX_USER_AGENT_LENGTH + 10);

        assert_eq!(
            clip_user_agent(&long).chars().count(),
            MAX_USER_AGENT_LENGTH
        );
    }
}
