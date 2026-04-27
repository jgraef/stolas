use std::path::PathBuf;

use axum::{
    Router,
    extract::{
        MatchedPath,
        Request,
        WebSocketUpgrade,
        ws::WebSocket,
    },
    response::{
        IntoResponse,
        Redirect,
    },
    routing,
};
use color_eyre::eyre::{
    Error,
    bail,
};
use futures_util::future;
use tower_http::{
    normalize_path::NormalizePathLayer,
    services::ServeDir,
    trace::{
        DefaultOnRequest,
        DefaultOnResponse,
        TraceLayer,
    },
};

use crate::{
    api::Api,
    station::Station,
};

pub async fn router(station: &Station) -> Result<Router<()>, Error> {
    let webui_path = webui_path()?;
    tracing::debug!(?webui_path);

    let api = Api::new(station);

    let router = Router::new()
        // redirect / to /ui
        .route(
            "/",
            routing::any(async || Redirect::temporary("/ui").into_response()),
        )
        // serve /ui from static files
        .nest_service(
            "/ui",
            ServeDir::new(webui_path).append_index_html_on_directories(true),
        )
        // serve /api/v1 with API
        .nest("/api/v1", api.into_router())
        // router to send reload signals
        .route(
            "/_dev/reload",
            routing::get(move |ws: WebSocketUpgrade| reload_signals(ws)),
        )
        // normalize paths
        .layer(NormalizePathLayer::trim_trailing_slash())
        // logging layer
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request| {
                    let method = req.method();
                    let uri = req.uri();

                    // axum automatically adds this extension.
                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(|matched_path| matched_path.as_str());

                    tracing::info_span!("request", %method, %uri, matched_path)
                })
                .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        );

    Ok(router)
}

async fn reload_signals(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(async move |_socket: WebSocket| {
        // just wait forever. when the server shutsdown the websocket will be closed,
        // causing the browser to reload the page.
        future::pending::<()>().await;
    })
}

fn webui_path() -> Result<PathBuf, Error> {
    if let Ok(path) = std::env::var("STOLAS_WEBUI") {
        let path = PathBuf::from(path);
        if !path.exists() {
            bail!("Path does not exist: {path:?}");
        }
        Ok(path)
    }
    else {
        let path = PathBuf::from("target/stolas-webui/dist/");
        if !path.exists() {
            bail!("webui not found. Please set the STOLAS_WEBUI environment variable.");
        }
        Ok(path)
    }
}
