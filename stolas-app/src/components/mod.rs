pub mod icon;
pub mod nav;
pub mod tooltip;

use dioxus::{
    document::{
        Script,
        Stylesheet,
    },
    prelude::*,
};

#[component]
pub fn BootstrapImport(#[props(optional)] cdn: bool) -> Element {
    const BOOTSTRAP_ASSET: Asset = asset!(
        "/assets/bootstrap",
        AssetOptions::folder().with_hash_suffix(false)
    );

    if cdn {
        rsx! {
            Script {
                src: "https://cdn.jsdelivr.net/npm/bootstrap@5.3.8/dist/js/bootstrap.bundle.min.js",
                integrity: "sha384-FKyoEForCGlyvwx9Hj09JcYn3nv7wiPVlz7YYwJrWVcXK/BmnVDxM+D2scQbITxI",
                crossorigin: "anonymous",
            }

            Stylesheet {
                href: "https://cdn.jsdelivr.net/npm/bootstrap@5.3.8/dist/css/bootstrap.min.css",

                integrity: "sha384-sRIl4kxILFvY47J16cr9ZwB07vP4J8+LH7qKQnuqkuIAvNWLzeN8tE5YBujZqJLB",

                crossorigin: "anonymous",
            }


            Stylesheet { href: "https://cdn.jsdelivr.net/npm/bootstrap-icons@1.13.1/font/bootstrap-icons.min.css" }
        }
    }
    else {
        rsx! {
            Script {
                src: format!("{BOOTSTRAP_ASSET}/bootstrap.bundle.min.js"),
                crossorigin: "anonymous",
            }
            Stylesheet { href: format!("{BOOTSTRAP_ASSET}/bootstrap.min.css") }
            Stylesheet { href: format!("{BOOTSTRAP_ASSET}/bootstrap-icons.min.css") }
        }
    }
}
