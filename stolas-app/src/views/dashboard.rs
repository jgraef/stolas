use dioxus::{
    document::eval,
    prelude::*,
};
use dioxus_leaflet::{
    LatLng,
    Map,
    MapOptions,
    MapPosition,
    Marker,
};
use dioxus_sdk::geolocation::{
    self,
    Geocoordinates,
};
use stolas_core::geo::GeoCoords;

use crate::components::{
    icon::Icon,
    tooltip::Tooltip,
};

#[component]
pub fn DashboardView() -> Element {
    let mut show_location = use_signal(|| true);

    // Modal to set location via geolocation API
    // todo: move this into a component/view

    // todo: init_geolocator requests permissions. we should only do this once the
    // user wants to use this feature.
    //
    // init_geolocator(PowerMode::High);
    // let geolocation_coords = use_geolocation();
    //
    // fallback: http://ip-api.com/json
    let geolocation_coords =
        use_signal(|| Err::<Geocoordinates, _>(geolocation::Error::Unsupported));

    rsx! {
        div { class: "modal", tabindex: "-1", id: "phone_geolocation_modal",
            div { class: "modal-dialog",
                div { class: "modal-content",
                    div { class: "modal-header",
                        h5 { class: "modal-title", "Set location from phone" }
                        button {
                            aria_label: "Close",
                            class: "btn-close",
                            "data-bs-dismiss": "modal",
                            r#type: "button",
                        }
                    }
                    div { class: "modal-body",
                        // note: this doesn't accept signals, but their docs say it's fine ^.^
                        {
                            match geolocation_coords.read().clone() {
                                Ok(coords) => {
                                    let coords = GeoCoords {

                                        latitude: coords.latitude,
                                        longitude: coords.longitude,
                                    };
                                    rsx! {
                                        p { "{coords.format()}" }
                                        Map {
                                            initial_position: MapPosition::new(coords.latitude, coords.longitude, 15.0),
                                            class: "location-map",
                                            options: MapOptions::default(),
                                            Marker { coordinate: LatLng::new(coords.latitude, coords.longitude) }
                                        }
                                    }
                                }
                                Err(error) => {
                                    let message = match error {
                                        geolocation::Error::NotInitialized
                                        | geolocation::Error::Poisoned => {
                                            rsx! { "Waiting for geolocation..." }
                                        }
                                        geolocation::Error::AccessDenied => {
                                            rsx! { "Geolocation permissions denied." }
                                        }
                                        geolocation::Error::DeviceError(error) => {
                                            rsx! { "Device error: {error}" }
                                        }
                                        geolocation::Error::Unsupported => {
                                            rsx! { "Geolocation unsupported." }
                                        }
                                    };
                                    rsx! {
                                        p { {message} }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "modal-footer",
                        button {
                            class: "btn btn-secondary",
                            "data-bs-dismiss": "modal",
                            r#type: "button",
                            "Cancel"
                        }
                        button {
                            class: "btn btn-primary",
                            "data-bs-dismiss": "modal",
                            r#type: "button",
                            disabled: geolocation_coords.with(|result| result.is_err()),
                            onclick: move |_event| {
                                if let Ok(coords) = geolocation_coords.read().clone() {
                                    tracing::debug!(
                                        ? coords, "todo: send api request to set this as geolocation"
                                    );
                                } else {
                                    tracing::error!(
                                        "fixme: save last-known geolocation instead of using the live-value here."
                                    );
                                }
                            },
                            "Confirm"
                        }
                    }
                }
            }
        }

        div { class: "container px-2",
            div { class: "d-flex flex-row flex-wrap justify-content-evenly align-items-stretch gap-3",

                // Time
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

                // System status
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

                // Geolocation
                div { class: "card w-100",
                    h5 { class: "card-header",
                        Icon { icon: "geo-alt", class: "pe-2" }
                        "Location"
                    }
                    div { class: "card-body",
                        {
                            // todo: put this into a memo
                            let station_location = GeoCoords {
                                latitude: 51.505,
                                longitude: -0.09,
                            };
                            //let formatted_location = ;

                            rsx! {
                                if show_location() {
                                    p { "{station_location.format()}" }
                                    Map {
                                        initial_position: MapPosition::new(station_location.latitude, station_location.longitude, 10.0),
                                        class: "location-map",
                                        options: MapOptions::default().with_dragging(false),
                                        Marker { coordinate: LatLng::new(station_location.latitude, station_location.longitude) }
                                    }
                                } else {
                                    div { class: "location-map placeholder" }
                                }
                            }
                        }
                    }
                    div { class: "card-footer",
                        div { class: "d-flex flex-row gap-2",
                            // Button to grab phone geolocation
                            button {
                                r#type: "button",
                                class: "btn btn-primary btn-sm",
                                onclick: move |_| async move {
                                    tracing::debug!("TODO: set pointing from phone");

                                    // init_geolocator(PowerMode::High);
                                    show_modal("phone_geolocation_modal").await;
                                    tracing::debug!("geolocation started");
                                },
                                Icon { icon: "phone", class: "pe-1" }
                                "Set from phone"
                            }

                            // Switch to hide location
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

                // Pointing
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

async fn show_modal(id: &'static str) {
    let _ = eval(&format!(
        r#"
        let element = document.getElementById('{id}');
        let modal = new bootstrap.Modal(element);
        modal.show();
        "#,
    ))
    .await;
}
