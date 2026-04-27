pub mod app;

use tracing::Level;
use tracing_wasm::WASMLayerConfigBuilder;
use wasm_bindgen::JsCast;

use crate::app::App;

fn main() {
    tracing_wasm::set_as_global_default_with_config(
        WASMLayerConfigBuilder::new()
            .set_max_level(Level::DEBUG)
            .build(),
    );
    console_error_panic_hook::set_once();

    tracing::info!("starting app");

    let root = web_sys::window()
        .expect("no window")
        .document()
        .expect("no document")
        .get_element_by_id("root")
        .expect("no root element")
        .dyn_into()
        .unwrap();

    let handle = leptos::mount::mount_to(root, App);
    handle.forget();
}
