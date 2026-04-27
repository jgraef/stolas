use dioxus::prelude::*;

use crate::Route;

#[component]
pub fn NavBar() -> Element {
    let brand = "Stolas";

    rsx! {
        main { "data-bs-theme": "dark", class: "bg-body-tertiary",
            nav { class: "navbar navbar-expand-lg bg-gradient shadow",
                div { class: "container-fluid",
                    Link { class: "navbar-brand", to: Route::DashboardView,
                        b { {brand} }
                    }
                    button {
                        aria_controls: "main_navbar",
                        aria_label: "Toggle navigation",
                        class: "navbar-toggler",
                        "data-bs-target": "#main_navbar",
                        "data-bs-toggle": "offcanvas",
                        r#type: "button",
                        span { class: "navbar-toggler-icon" }
                    }
                    div {
                        class: "offcanvas offcanvas-start",
                        id: "main_navbar",
                        tabindex: "-1",
                        div { class: "offcanvas-header",
                            h5 { class: "offcanvas-title", {brand} }
                            button {
                                r#type: "button",
                                class: "btn-close",
                                "data-bs-dismiss": "offcanvas",
                                aria_label: "Close",
                            }
                        }
                        div { class: "offcanvas-body",
                            ul { class: "navbar-nav me-auto mb-2 mb-lg-0",
                                for item in NAV_ITEMS {
                                    li { class: "nav-item",
                                        Link {
                                            active_class: "active",
                                            class: "nav-link",
                                            to: item.to.clone(),
                                            "{item.label}"
                                        }
                                    }
                                }
                            }

                        }
                    }
                }
            }

            div { class: "container py-4 me-auto", Outlet::<Route> {} }

        }
    }
}

pub struct NavItem<'a> {
    pub to: Route,
    pub label: &'a str,
}

pub const NAV_ITEMS: &[NavItem] = &[
    NavItem {
        to: Route::DashboardView,
        label: "Dashboard",
    },
    NavItem {
        to: Route::CaptureView,
        label: "Capture",
    },
    NavItem {
        to: Route::SettingsView,
        label: "Settings",
    },
];
