use std::{
    fmt::Debug,
    path::Path,
};

use tokio::process::Command;

use crate::util::process::{
    ExitStatusError,
    ExitStatusExt,
};

pub async fn wasm_bindgen(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    output_name: &str,
) -> Result<(), WasmBindgenError> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();

    if let Err(error) = wasm_bindgen_bin_test().await {
        tracing::error!(?error, "wasm-bindgen binary failed");
        tracing::error!(
            "You either need to install wasm-bindgen (`cargo install wasm-bindgen-cli`), or enable the `wasm-bindgen-lib` feature."
        );
        return Err(WasmBindgenError::BinNotFound);
    }
    else {
        wasm_bindgen_bin(input_path, output_path, output_name).await?;
    }

    Ok(())
}

async fn wasm_bindgen_bin(
    input_path: &Path,
    output_dir: &Path,
    output_name: &str,
) -> Result<(), WasmBindgenError> {
    Command::new("wasm-bindgen")
        .arg("--out-dir")
        .arg(output_dir)
        .arg("--out-name")
        .arg(output_name)
        .arg("--target")
        .arg("web")
        .arg("--no-typescript")
        .arg(input_path)
        .spawn()?
        .wait()
        .await?
        .into_result()?;
    Ok(())
}

async fn wasm_bindgen_bin_test() -> Result<(), WasmBindgenError> {
    Command::new("wasm-bindgen")
        .arg("--version")
        .spawn()?
        .wait()
        .await?
        .into_result()?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("wasm-bindgen error")]
pub enum WasmBindgenError {
    BinNotFound,
    Io(#[from] std::io::Error),
    ExitStatus(#[from] ExitStatusError),
}
