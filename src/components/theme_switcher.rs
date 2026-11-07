use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        HeaderName,
        content::Form,
        error::{SeeOther, bad_request, see_other},
        header::SET_COOKIE,
        href, request, route,
    },
    runtime::Event,
    view::{View, component, view},
};

use super::icon::{IconName, icon};
use crate::theme::Theme;
use crate::web::globals;

fn theme_icon(theme: Theme) -> IconName {
    match theme {
        Theme::Light => IconName::Sun,
        Theme::Dark => IconName::Moon,
        Theme::System => IconName::Monitor,
    }
}

#[component]
pub async fn theme_switcher(cx: &Cx, #[default] compact: bool) -> Result<impl View> {
    let current = globals(cx).theme;
    let return_to = request::uri(cx)
        .path_and_query()
        .map_or_else(|| "/".to_string(), |path| path.as_str().to_string());
    let class = if compact {
        "a-theme-switcher a-theme-switcher--compact"
    } else {
        "a-theme-switcher"
    };

    Ok(view! {
        <form
            class=(class)
            method="post"
            action=(href!(set_theme))
            role="group"
            aria-label="Color theme"
        >
            <input type="hidden" name="return_to" value=(return_to)>
            #[key(theme.as_str())]
            for theme in Theme::ALL {
                <button
                    type="submit"
                    class="a-theme-switcher__option"
                    name="theme"
                    value=(theme.as_str())
                    title=(theme.label())
                    aria-label=(theme.label())
                    aria-pressed=(if theme == current { "true" } else { "false" })
                    @click=$(|e: Event| {
                        e.prevent_default();
                        let _selected = e.current_target.value;
                        raw!(
                            "(() => {
                            const theme = String(${_selected});
                            document.documentElement.dataset.theme = theme;
                            document.querySelector('meta[name=color-scheme]').content = theme === 'system' ? 'light dark' : theme;
                            document.cookie = 'theme=' + theme + '; path=/; max-age=31536000; samesite=lax';
                            for (const option of document.querySelectorAll('.a-theme-switcher__option')) {
                                option.setAttribute('aria-pressed', String(option.value === theme));
                            }
                        })()",
                            (),
                        )
                    })
                >
                    icon(name: theme_icon(theme))
                    if !compact {
                        <span class="a-theme-switcher__label">(theme.label())</span>
                    }
                </button>
            }
        </form>
    })
}

#[derive(Deserialize)]
pub struct ThemeInput {
    theme: String,
    return_to: String,
}

#[route(POST "/theme")]
pub async fn set_theme(
    Form(input): Form<ThemeInput>,
) -> Result<([(HeaderName, String); 1], SeeOther)> {
    let theme = Theme::from_slug(&input.theme).ok_or_else(|| bad_request("Unknown theme."))?;

    Ok((
        [(SET_COOKIE, theme.set_cookie())],
        see_other(safe_return_path(&input.return_to)),
    ))
}

fn safe_return_path(path: &str) -> &str {
    let is_local = path.starts_with('/') && !path.starts_with("//") && !path.contains('\\');
    if is_local { path } else { "/" }
}
