use std::time::Duration;

use color_eyre::eyre::{
    Error,
    bail,
};
use tokio::process::Command;

use crate::{
    util::watch::WatchFiles,
    webui::build_ui,
};

pub async fn dev(watch: bool) -> Result<(), Error> {
    build_ui("./stolas-webui", "target/stolas-webui/dist", false, false).await?;

    if watch {
        let mut watch_files = WatchFiles::new()?;
        watch_files.watch("./stolas-webui")?;

        tokio::spawn(async move {
            while let Some(_changes) = watch_files.next(Some(Duration::from_secs(5))).await {
                tracing::info!("Changes to webui detected. Rebuilding");
                if let Err(error) =
                    build_ui("./stolas-webui", "target/stolas-webui/dist", false, false).await
                {
                    tracing::error!(%error, "webui build failed");
                }
                // todo: would be nice to somehow send a signal to the server,
                // so it can make the browser reload
            }
        });
    }

    tracing::info!("Starting station server");
    run_station().await?;

    Ok(())
}

pub async fn run_station() -> Result<(), Error> {
    let exit_status = Command::new("cargo")
        .args(["run", "--bin", "stolas-station"])
        .kill_on_drop(true)
        .spawn()?
        .wait()
        .await?;

    if !exit_status.success() {
        bail!("ssh failed: {exit_status:?}");
    }

    Ok(())
}
