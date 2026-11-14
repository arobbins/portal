use crate::syntax::{CodeLanguage, highlight};
use topcoat::{
    Result,
    context::Cx,
    runtime::{Event, signal},
    view::{Child, View, component, view},
};

#[component]
pub async fn code_panel(
    cx: &Cx,
    title: &str,
    #[default] label: Option<&str>,
    #[default] code_title: bool,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    let title_class = if code_title {
        "a-code-panel__title a-code-panel__title--code"
    } else {
        "a-code-panel__title"
    };

    Ok(view! {
        <div class="a-code-panel">
            <p class="a-code-panel__header">
                <span class=(title_class)>(title)</span>
                if let Some(label) = label {
                    <span class="a-code-panel__label">(label)</span>
                }
            </p>
            (child)
        </div>
    })
}

#[component]
pub async fn code_block(
    cx: &Cx,
    code: &str,
    #[default] language: CodeLanguage,
    #[default] line_numbers: bool,
) -> Result<impl View> {
    let lines = highlight(code, language);
    let last_line = lines.len().saturating_sub(1);
    let pre_class = if line_numbers {
        "a-code-block__pre a-code-block__pre--numbered"
    } else {
        "a-code-block__pre"
    };
    let copied = signal(cx, || false);
    let copy_value = code.to_string();
    let copy_ok: std::result::Result<bool, bool> = Ok(true);
    let copy_failed: std::result::Result<bool, bool> = Err(false);

    Ok(view! {
        <div class="a-code-block">
            <pre class=(pre_class)>
                <code class="a-code-block__code">
                    #[key(index)]
                    for (index, line) in lines.iter().enumerate() {
                        <span class="a-code-block__line">
                            #[key(position)]
                            for (position, token) in line.iter().enumerate() {
                                <span class=(token.kind.class())>
                                    (token.text.as_str())
                                </span>
                            }
                        </span>
                        if index < last_line {
                            "\n"
                        }
                    }
                </code>
            </pre>
            <button
                type="button"
                class="a-code-block__copy"
                aria-label="Copy code"
                @click=$(async |_e: Event| {
                    let result = raw!(
                        "await navigator.clipboard.writeText(String(${copy_value})).then(() => ${copy_ok}, () => ${copy_failed})",
                        copy_failed,
                    );
                    copied.set(result.is_ok());
                    let _waited = raw!(
                        "await new Promise((resolve) => setTimeout(resolve, 2000))",
                        (),
                    );
                    copied.set(false);
                })
            >
                $(if copied.get() { "Copied" } else { "Copy" })
            </button>
            <span class="u-visually-hidden" role="status">
                $(if copied.get() { "Copied to clipboard" } else { "" })
            </span>
        </div>
    })
}
