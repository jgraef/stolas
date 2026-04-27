pub mod components;
pub mod views;

use dioxus::{
    document::Script,
    prelude::*,
};

use crate::{
    components::nav::NavBar,
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

    const BOOTSTRAP_ICONS: Asset = asset!("/assets/bootstrap-icons");

    rsx! {
        Script { src: asset!("/assets/bootstrap.bundle.min.js", AssetOptions::js().with_minify(false)) }
        Stylesheet { href: asset!("/assets/main.css") }
        Stylesheet { href: asset!("/assets/bootstrap.min.css", AssetOptions::css().with_minify(false)) }
        Stylesheet { href: format!("{BOOTSTRAP_ICONS}/bootstrap-icons.min.css") }

        Router::<Route> {}
    }

    //Stylesheet { href: "https://cdn.jsdelivr.net/npm/bootstrap-icons@1.13.1/font/bootstrap-icons.min.css" }
    //Script {
    //    src: "https://cdn.jsdelivr.net/npm/bootstrap@5.3.8/dist/js/bootstrap.bundle.min.js",
    //    integrity:
    // "sha384-FKyoEForCGlyvwx9Hj09JcYn3nv7wiPVlz7YYwJrWVcXK/BmnVDxM+D2scQbITxI"
    // ,    crossorigin: "anonymous",
    //}
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
