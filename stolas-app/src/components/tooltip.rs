use std::sync::atomic::{
    AtomicUsize,
    Ordering,
};

use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum TooltipPlacement {
    Top,
    Right,
    #[default]
    Bottom,
    Left,
}

/// # FIXME
///
/// When the page loads the `bootstrap` variable is still undefined and thus
/// tooltips will not be initialized. We don't know how to fix this at the
/// moment.
#[component]
pub fn Tooltip(
    #[props(default)] placement: TooltipPlacement,
    class: Option<String>,
    text: String,
    children: Element,
) -> Element {
    let placement = match placement {
        TooltipPlacement::Top => "top",
        TooltipPlacement::Right => "right",
        TooltipPlacement::Bottom => "bottom",
        TooltipPlacement::Left => "left",
    };

    let tooltip_id = {
        static IDS: AtomicUsize = AtomicUsize::new(1);
        IDS.fetch_add(1, Ordering::Relaxed)
    };

    rsx! {
        div {
            class: "d-inline",
            "data-bs-toggle": "tooltip",
            "data-bs-placement": placement,
            "data-bs-custom-class": class,
            "data-tooltip-id": tooltip_id,
            "data-bs-title": text,
            onmount: move |_event| {
                //let element = event.as_web_event();
                spawn(async move {
                    tracing::debug!("initializing tooltips");
                    document::eval(&init_tooltip_js(tooltip_id)).await.unwrap();
                });
            },
            {children}
        }
    }
}

fn init_tooltip_js(id: usize) -> String {
    format!(
        r#"
        if (typeof bootstrap !== "undefined") {{
            console.log("init {id}");
            const tooltipTriggerList = document.querySelectorAll('[data-tooltip-id="{id}"]');
            const tooltipList = [...tooltipTriggerList].map(tooltipTriggerEl => new bootstrap.Tooltip(tooltipTriggerEl));
            return true;
        }}
        else {{
            console.log("could not initialize tooltip {id}");
            return false;
        }}
        "#,
    )
}
