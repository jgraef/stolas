use tokio_util::sync::CancellationToken;

pub async fn sigterm() {
    #[cfg(unix)]
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .unwrap()
        .recv()
        .await;

    #[cfg(not(unix))]
    std::future::pending::<()>().await;
}

pub fn cancel_on_ctrl_c_or_sigterm(token: CancellationToken) {
    tokio::spawn(async move {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl-C. Shutting down.");
                token.cancel();
            },
            _ = sigterm() => {
                tracing::info!("Received SIGTERM. Shutting down.");
                token.cancel();
            },
            _ = token.cancelled() => {}
        }
    });
}
