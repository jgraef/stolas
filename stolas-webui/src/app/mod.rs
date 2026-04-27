pub mod observe;
pub mod status;

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
use leptos_router::{
    components::{
        A,
        Route,
        Router,
        Routes,
    },
    path,
};
use leptos_styling::{
    StyleSheets,
    style_sheet,
};
use url::Url;

use crate::app::{
    observe::ObservePage,
    status::StatusPage,
};

fn get_base_url() -> Option<Url> {
    gloo_utils::document().base_uri().ok()??.parse().ok()
}

pub fn default_api_url() -> Url {
    let base_url: Url = get_base_url().expect("could not determine base URL");
    let api_url = base_url.join("api/v1").unwrap();
    tracing::debug!(%api_url);
    api_url
}

style_sheet!(app, "src/app/app.scss", "app");

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    let _api_url = default_api_url();
    //let api_client = ApiClient::new(api_url);
    //provide_context(api_client);

    spawn_local(handle_reload());

    view! {
        <Link rel="icon" type_="image/x-icon" href="/static/favicon.png" />
        <StyleSheets />
        <Router base="/ui">
            <nav class="navbar navbar-expand-md navbar-dark bg-dark" aria-label="Navbar">
                <div class="container-fluid">
                    <a class="navbar-brand" href="#">"Stolas"</a>
                    <button class="navbar-toggler" type="button" data-bs-toggle="collapse" data-bs-target="#top_navbar" aria-controls="top_navbar" aria-expanded="false" aria-label="Toggle navigation">
                        <span class="navbar-toggler-icon"></span>
                    </button>
                    <div class="collapse navbar-collapse" id="top_navbar">
                        <ul class="navbar-nav me-auto mb-2 mb-md-0">
                            <li class="nav-item">
                                <a class="nav-link active" aria-current="page" href="#">"Dashboard"</a>
                            </li>
                            <li class="nav-item">
                                <a class="nav-link" href="#">"Link"</a>
                            </li>
                            <li class="nav-item">
                                <a class="nav-link disabled" aria-disabled="true">"Disabled"</a>
                            </li>
                            <li class="nav-item dropdown">
                                <a class="nav-link dropdown-toggle" href="#" data-bs-toggle="dropdown" aria-expanded="false">Dropdown</a>
                                <ul class="dropdown-menu">
                                    <li>
                                        <a class="dropdown-item" href="#">Action</a>
                                    </li>
                                    <li>
                                        <a class="dropdown-item" href="#">Another action</a>
                                    </li>
                                    <li>
                                        <a class="dropdown-item" href="#">Something else here</a>
                                    </li>
                                </ul>
                            </li>
                        </ul>
                        <form role="search">
                            <input class="form-control" type="search" placeholder="Search" aria-label="Search" />
                        </form>
                    </div>
                </div>
            </nav>
            /*<div>
                <nav>
                    <div>
                         <A href="/">"Status"</A>
                         <A href="/observe">"Observe"</A>
                    </div>
                </nav>
                <main>
                    <Routes fallback=PageNotFound>
                        <Route path=path!("/") view=StatusPage />
                        <Route path=path!("/observe") view=ObservePage />
                    </Routes>
                </main>
            </div>*/
        </Router>
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
