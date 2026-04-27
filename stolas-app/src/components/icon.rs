use dioxus::prelude::*;

#[component]
pub fn Icon(
    icon: String,
    color: Option<String>,
    class: Option<String>,
    style: Option<String>,
) -> Element {
    let mut merged_class = format!("bi bi-{icon} ");
    if let Some(class) = class {
        merged_class.push_str(&class);
    }

    rsx! {
        i { class: merged_class, style }
    }
}
