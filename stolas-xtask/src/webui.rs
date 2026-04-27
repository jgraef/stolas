use std::{
    fs::File,
    io::{
        BufReader,
        BufWriter,
    },
    path::Path,
};

use chrono::{
    DateTime,
    Utc,
};
use color_eyre::eyre::Error;
use serde::{
    Deserialize,
    Serialize,
};
use tera::Tera;

use crate::util::{
    cargo::Cargo,
    git::Git,
    path_modified_timestamp,
    wasm_bindgen::wasm_bindgen,
};

#[tracing::instrument(skip_all)]
pub async fn build_ui(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    clean: bool,
    release: bool,
) -> Result<(), Error> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();

    tracing::info!(
        ?input_path,
        ?output_path,
        ?clean,
        ?release,
        "Building webui"
    );

    std::fs::create_dir_all(&output_path)?;

    let cargo = Cargo::new(&input_path);

    let manifest = cargo.manifest().await?;
    if manifest.targets.len() != 1 {
        // todo: don't panic
        panic!("Unexpected number of targets: {}", manifest.targets.len());
    }

    let build_time = Utc::now();
    let build_info_path = output_path.join("build_info.json");
    let build_info = if build_info_path.exists() && !clean {
        let reader = BufReader::new(File::open(&build_info_path)?);
        let build_info: BuildInfo = serde_json::from_reader(reader)?;
        Some(build_info)
    }
    else {
        None
    };

    let commit = Git.head().await.ok();

    let target_name = &manifest.targets[0].name;
    tracing::debug!(%target_name);

    let workspace_path = cargo.locate_workspace().await?;
    let workspace_path = workspace_path.parent().unwrap();
    tracing::debug!(workspace_path = %workspace_path.display());

    let profile = if release { "release" } else { "debug" };
    let target_wasm_path = workspace_path
        .join("target")
        .join("wasm32-unknown-unknown")
        .join(profile)
        .join(format!("{target_name}.wasm"));
    tracing::debug!(target_wasm_path = %target_wasm_path.display());

    let wasm_filename = format!("{target_name}_bg.wasm");
    let js_filename = format!("{target_name}.js");
    let index_filename = "index.html";

    // check if all files exist
    if !output_path.join(&wasm_filename).exists()
        || !output_path.join(&js_filename).exists()
        || !output_path.join(&index_filename).exists()
    {
        tracing::warn!("input file missing. rebuilding.");
    }
    else {
        // check freshness
        let input_modified_time = path_modified_timestamp(input_path, std::cmp::max)?;
        let previous_build_time = build_info.as_ref().map(|build_info| build_info.build_time);

        tracing::debug!(?input_modified_time, ?previous_build_time);

        let is_fresh = match (input_modified_time, previous_build_time) {
            (None, _) => true,
            (Some(input_modified_time), Some(output_modified_time))
                if input_modified_time <= output_modified_time =>
            {
                true
            }
            _ => false,
        };

        if is_fresh {
            tracing::debug!("not modified since last build. skipping.");
            return Ok(());
        }
    }

    tracing::info!(target = %target_name, "running `cargo build`");
    cargo.build(Some("wasm32-unknown-unknown"), release).await?;

    tracing::info!(target = %target_name, "running `wasm-bindgen`");
    wasm_bindgen(&target_wasm_path, output_path, &target_name).await?;

    tracing::debug!(target = %target_name, "generating `index.html`");
    let mut writer = BufWriter::new(File::create(output_path.join(&index_filename))?);

    let mut tera = Tera::default();
    tera.add_raw_template(
        "index",
        &std::fs::read_to_string(&input_path.join("index.html"))?,
    )?;
    tera.render_to(
        "index",
        &tera::Context::from_serialize(IndexHtml {
            js: &js_filename,
            wasm: &wasm_filename,
            title: &format!("Stolas (build {:?})", build_time),
        })?,
        &mut writer,
    )?;

    let build_info = BuildInfo {
        build_time,
        version: manifest.version,
        commit,
    };

    let writer = BufWriter::new(File::create(&build_info_path)?);
    serde_json::to_writer_pretty(writer, &build_info)?;

    tracing::info!("done");

    Ok(())
}

#[derive(Debug, Serialize)]
struct IndexHtml<'a> {
    js: &'a str,
    wasm: &'a str,
    title: &'a str,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BuildInfo {
    build_time: DateTime<Utc>,
    version: String,
    commit: Option<String>,
}
