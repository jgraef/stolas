use color_eyre::eyre::{
    Error,
    bail,
};
use tokio::process::Command;

pub async fn deploy(host: &str, path: &str, run: bool) -> Result<(), Error> {
    let destination = format!("{host}:{path}");

    tracing::info!(destination, "Copying source tree to telescope");

    let exit_status = Command::new("rsync")
        .args([
            "-raP",
            "--cvs-exclude",
            "--exclude-from",
            ".gitignore",
            ".",
            &destination,
        ])
        .kill_on_drop(true)
        .spawn()?
        .wait()
        .await?;

    if !exit_status.success() {
        bail!("rsync failed: {exit_status:?}");
    }

    if run {
        let command = format!(
            "cd {} && cargo run --bin stolas-telescope --release",
            shell_escape::escape(path.into())
        );
        tracing::info!(command, "Running deployed code");

        let exit_status = Command::new("ssh")
            .args([host, "sh", "-c", &shell_escape::escape(command.into())])
            .kill_on_drop(true)
            .spawn()?
            .wait()
            .await?;

        if !exit_status.success() {
            bail!("ssh failed: {exit_status:?}");
        }
    }

    Ok(())
}
