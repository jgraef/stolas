pub mod observe;
pub mod status;
pub mod util;

use futures_util::TryStreamExt;
use leptos::{
    IntoView,
    component,
    prelude::*,
    task::spawn_local,
    view,
};
use leptos_meta::{
    Link,
    provide_meta_context,
};
use url::Url;

fn get_base_url() -> Option<Url> {
    gloo_utils::document().base_uri().ok()??.parse().ok()
}

pub fn default_api_url() -> Url {
    let base_url: Url = get_base_url().expect("could not determine base URL");
    let api_url = base_url.join("api/v1").unwrap();
    tracing::debug!(%api_url);
    api_url
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let _api_url = default_api_url();
    //let api_client = ApiClient::new(api_url);
    //provide_context(api_client);

    spawn_local(handle_reload());

    view! {
        <Link rel="icon" type_="image/x-icon" href="/static/favicon.png" />
        <h1>"Hello World"</h1>
    }
}

#[component]
fn PageNotFound() -> impl IntoView {
    view! {
        <div>
            <h1>"404 - Page not found"</h1>
        </div>
    }
}

async fn handle_reload() {
    let base_url: Url = get_base_url().expect("could not determine base URL");
    let reload_url = base_url.join("_dev/reload").unwrap();

    if let Ok(mut socket) = reqwest_websocket::websocket(reload_url).await {
        let _ = socket.try_next().await;
        let _ = web_sys::window().unwrap().location().reload();
    }
}
