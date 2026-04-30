pub mod components;
pub mod views;

use dioxus::prelude::*;

use crate::{
    components::{
        BootstrapImport,
        nav::NavBar,
    },
    views::{
        capture::CaptureView,
        dashboard::DashboardView,
        not_found::PageNotFound,
        settings::SettingsView,
    },
};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // fixme: window.bootstrap is undefined when this runs, but it is defined later
    // (check dev console)
    /*use_effect(move || {
        spawn(async {
            tracing::debug!("initializing tooltips");
            document::eval(
            r#"
            const tooltipTriggerList = document.querySelectorAll('[data-bs-toggle="tooltip"]')
            const tooltipList = [...tooltipTriggerList].map(tooltipTriggerEl => new bootstrap.Tooltip(tooltipTriggerEl))
            "#
            ).await.unwrap();
        });
    });*/

    rsx! {
        Stylesheet { href: asset!("/assets/main.css") }
        BootstrapImport { cdn: false }

        Router::<Route> {}
    }
}

#[derive(Clone, PartialEq, Routable)]
pub enum Route {
    #[layout(NavBar)]
    #[route("/")]
    DashboardView,
    #[route("/capture")]
    CaptureView,
    #[route("/settings")]
    SettingsView,
    #[route("/:..segments")]
    PageNotFound { segments: Vec<String> },
}
