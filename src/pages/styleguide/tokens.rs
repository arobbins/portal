use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use super::styleguide_section;

const VARIABLES: &str = include_str!("../../../assets/css/variables.css");

struct Scale {
    title: &'static str,
    prefix: &'static str,
    sample: &'static str,
    text: &'static str,
}

const SCALES: &[Scale] = &[
    Scale {
        title: "Colours",
        prefix: "color-",
        sample: "p-styleguide__swatch",
        text: "",
    },
    Scale {
        title: "Spacing",
        prefix: "space-",
        sample: "p-styleguide__space",
        text: "",
    },
    Scale {
        title: "Type",
        prefix: "text-",
        sample: "p-styleguide__type",
        text: "Aa",
    },
    Scale {
        title: "Radii",
        prefix: "radius-",
        sample: "p-styleguide__radius",
        text: "",
    },
];

fn token_names(prefix: &str) -> Vec<&'static str> {
    VARIABLES
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix("--"))
        .filter_map(|line| line.split_once(':').map(|(name, _)| name))
        .filter(|name| name.starts_with(prefix))
        .collect()
}

fn token_class(name: &str) -> String {
    format!("p-styleguide__token--{name}")
}

struct Token {
    variable: String,
    class: String,
}

fn tokens(scale: &Scale) -> Vec<Token> {
    token_names(scale.prefix)
        .into_iter()
        .map(|name| Token {
            variable: format!("--{name}"),
            class: format!("{} {}", scale.sample, token_class(name)),
        })
        .collect()
}

#[component]
pub async fn tokens_section(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        styleguide_section(
            title: "Tokens",
            for scale in SCALES {
                token_scale(scale: scale)
            }
        )
    })
}

#[component]
async fn token_scale(cx: &Cx, scale: &Scale) -> Result<impl View> {
    let tokens = tokens(scale);

    Ok(view! {
        <h3 class="p-styleguide__subheading">(scale.title)</h3>
        <ul class="p-styleguide__tokens">
            for token in &tokens {
                <li class="p-styleguide__token">
                    <span class=(&token.class) aria-hidden="true">(scale.text)</span>
                    <code>(&token.variable)</code>
                </li>
            }
        </ul>
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLEGUIDE_CSS: &str = include_str!("../../../assets/css/pages/styleguide-tokens.css");

    #[test]
    fn every_token_has_a_sample_class() {
        for scale in SCALES {
            let names = token_names(scale.prefix);
            assert!(!names.is_empty(), "no {} tokens", scale.prefix);
            for name in names {
                let selector = format!(".{} {{", token_class(name));
                assert!(STYLEGUIDE_CSS.contains(&selector), "missing {selector}");
            }
        }
    }
}
