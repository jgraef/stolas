use dioxus::prelude::*;
use dioxus_leaflet::{
    LatLng,
    Map,
    MapPosition,
    Marker,
};

use crate::components::{
    icon::Icon,
    tooltip::Tooltip,
};

#[component]
pub fn DashboardView() -> Element {
    let mut show_location = use_signal(|| true);

    /*
    todo: use bundled leaflet js/css
    options: MapOptions::default()
    .with_leaflet_resources(LeafletResources::Local {
        css_path: todo!(),
        js_path: todo!(),
    })
    */

    rsx! {
        div { class: "container px-2",
            div { class: "d-flex flex-row flex-wrap justify-content-evenly align-items-stretch gap-3",

                div { class: "card flex-fill",
                    h5 { class: "card-header",
                        Icon { icon: "clock", class: "pe-2" }
                        "Time"
                    }
                    div { class: "card-body",
                        p { class: "card-text text-nowrap", "2026-04-29 09:36:12 UTC" }
                    }
                    div { class: "card-footer",
                        Tooltip { text: "Hello World",
                            span { class: "badge text-bg-success", "sync" }
                        }
                    }
                }

                div { class: "card flex-fill",
                    h5 { class: "card-header",
                        Icon { icon: "motherboard", class: "pe-2" }
                        "System"
                    }
                    div { class: "card-body",
                        div { class: "container-flex",
                            div { class: "row",
                                div { class: "col", "CPU:" }
                                div { class: "col text-nowrap justify-end", "69 %" }
                            }
                            div { class: "row",
                                div { class: "col", "Memory:" }
                                div { class: "col text-nowrap justify-end", "1.23 GiB" }
                            }
                            div { class: "row",
                                div { class: "col", "Temperature:" }
                                div { class: "col text-nowrap justify-end", "22 °C" }
                            }
                        }
                    }
                }

                div { class: "card w-100",
                    h5 { class: "card-header",
                        Icon { icon: "geo-alt", class: "pe-2" }
                        "Location"
                    }
                    div { class: "card-body",
                        if show_location() {
                            Map {
                                initial_position: MapPosition::new(51.505, -0.09, 4.0),
                                class: "location-map",
                                Marker { coordinate: LatLng::new(51.505, -0.09) }
                            }
                        } else {
                            div { class: "location-map placeholder" }
                        }
                    }
                    div { class: "card-footer",
                        div { class: "d-flex flex-row gap-2",
                            button {
                                r#type: "button",
                                class: "btn btn-primary btn-sm",
                                onclick: move |_| { tracing::debug!("TODO: set pointing from phone") },
                                Icon { icon: "phone", class: "pe-1" }
                                "Set from phone"
                            }
                            div { class: "form-check form-switch",
                                input {
                                    class: "form-check-input",
                                    id: "hide_location",
                                    r#type: "checkbox",
                                    role: "switch",
                                    checked: show_location(),
                                    onclick: move |_| show_location.toggle(),
                                }
                                label {
                                    class: "form-check-label",
                                    r#for: "hide_location",
                                    "Show"
                                }
                            }
                        }
                    }
                }

                div { class: "card w-100",
                    h5 { class: "card-header",
                        Icon { icon: "compass", class: "pe-2" }
                        "Pointing"
                    }
                    div { class: "card-body",
                        div { class: "d-flex flex-row flex-wrap gap-2",
                            div { class: "d-inline-flex flex-column flex-grow-1 border rounded py-1 px-2",
                                b { "Horizontal" }
                                div { class: "row",
                                    div { class: "col", "Altitude:" }
                                    div { class: "col text-nowrap justify-end", "45 °" }
                                }
                                div { class: "row",
                                    div { class: "col", "Azimuth:" }
                                    div { class: "col text-nowrap justify-end", "180 °" }
                                }
                            }
                            div { class: "d-inline-flex flex-column flex-grow-1 border rounded py-1 px-2",
                                b { "Equatorial" }
                                div { class: "row",
                                    div { class: "col", "Declination:" }
                                    div { class: "col text-nowrap justify-end", "45 °" }
                                }
                                div { class: "row",
                                    div { class: "col", "Right Ascension:" }
                                    div { class: "col text-nowrap justify-end", "01h 31m 29.01s" }
                                }
                            }
                            div { class: "d-inline-flex flex-column flex-grow-1 border rounded py-1 px-2",
                                b { "Galactic" }
                                div { class: "row",
                                    div { class: "col", "Latitude:" }
                                    div { class: "col text-nowrap justify-end", "1 °" }
                                }
                                div { class: "row",
                                    div { class: "col", "Longitude:" }
                                    div { class: "col text-nowrap justify-end", "23 °" }
                                }
                            }
                        }
                    }
                    hr { class: "my-0" }
                    div { class: "card-body", StarMap {} }
                    div { class: "card-footer",
                        button {
                            r#type: "button",
                            class: "btn btn-primary btn-sm",
                            onclick: move |_| { tracing::debug!("TODO: set pointing from phone") },
                            Icon { icon: "phone", class: "pe-1" }
                            "Set from phone"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn StarMap() -> Element {
    rsx! {
        figure { class: "figure my-0",
            img {
                src: asset!("/assets/starmap_4k_galactic.jpg"),
                alt: "Galactic disk as seen from earth",
                class: "img-fluid figure-img",
            }
            figcaption { class: "figure-caption text-start",
                p { class: "small my-0",
                    "NASA/Goddard Space Flight Center Scientific Visualization Studio. Gaia DR2: ESA/Gaia/DPAC. Constellation figures based on those developed for the IAU by Alan MacRobert of Sky and Telescope magazine (Roger Sinnott and Rick Fienberg)."
                    a {
                        class: "icon-link ms-1",
                        href: "https://svs.gsfc.nasa.gov/4851",
                        Icon { icon: "box-arrow-up-right" }
                    }
                }
            }
        }
    }
}
