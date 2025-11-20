mod cli;
mod extract;
mod github;
mod manifest;
mod move_files;
mod permissions;

use crate::cli::BootstrapperArgs;
use crate::extract::extract_7z;
use crate::github::{download_latest_obs_build, get_latest_obs_build_info};
use crate::manifest::{
    get_all_files_relative, remove_manifest_files, write_manifest, BootstrapManifest,
};
use crate::move_files::move_all_up_one_level;
use crate::permissions::is_writable;
use clap::Parser;
use std::env;
use std::fs;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = BootstrapperArgs::parse();
    let exe_dir = if std::env::var_os("BOOTSTRAPPER_TEST_MODE").is_some() {
        std::env::current_dir()?
    } else {
        env::current_exe()?.parent().unwrap().to_path_buf()
    };

    // Cleanup mode: only remove manifest and files, then exit
    if args.cleanup {
        let manifest_path = exe_dir.join("bootstrap_manifest.json");
        if manifest_path.exists() {
            let manifest: BootstrapManifest =
                serde_json::from_reader(fs::File::open(&manifest_path)?)?;
            remove_manifest_files(&exe_dir, &manifest)?;
            fs::remove_file(&manifest_path)?;
            println!("Cleanup complete.");
        } else {
            println!("No manifest found, nothing to clean.");
        }
        return Ok(());
    }

    // Get latest version info from GitHub (defer sha256 download)
    let (version, asset_path, sha256_url) = get_latest_obs_build_info().await?;
    let manifest_path = exe_dir.join("bootstrap_manifest.json");
    if manifest_path.exists() {
        let manifest: BootstrapManifest = serde_json::from_reader(fs::File::open(&manifest_path)?)?;
        // If version matches and not forced, exit early
        if manifest.version == version && !args.force {
            println!("no update required");
            std::process::exit(0);
        }
    }

    // If we lack permission to write to the installation directory, fail fast — Tauri is expected
    // to launch the bootstrapper elevated when necessary.
    if !is_writable(&exe_dir) {
        anyhow::bail!("Insufficient permissions to write to installation directory; run as administrator or launch the app which will elevate the downloader.");
    }

    // Remove files from manifest if present (after PID wait)
    if manifest_path.exists() {
        let manifest: BootstrapManifest = serde_json::from_reader(fs::File::open(&manifest_path)?)?;
        remove_manifest_files(&exe_dir, &manifest)?;
        fs::remove_file(&manifest_path)?;
    }

    // Download sha256 only now
    let sha256_file = reqwest::get(&sha256_url).await?.text().await?;
    let sha256 = sha256_file
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string();
    if sha256.is_empty() {
        anyhow::bail!("obs-build.7z hash not found in sha256 file");
    }
    // Download and verify obs-build.7z
    let obs_build_path = exe_dir.join("obs-build.7z");
    download_latest_obs_build(&asset_path, &sha256, &obs_build_path).await?;

    // Extract to bootstrap dir
    let bootstrap_dir = exe_dir.join("bootstrap");
    extract_7z(&obs_build_path, &bootstrap_dir)?;
    fs::remove_file(&obs_build_path)?;

    // Write new manifest
    let files = get_all_files_relative(&bootstrap_dir)?;
    let manifest = BootstrapManifest {
        version,
        files: files.clone(),
    };
    write_manifest(&manifest_path, &manifest)?;

    // Move files up one level
    move_all_up_one_level(&bootstrap_dir, &exe_dir)?;
    fs::remove_dir_all(&bootstrap_dir)?;

    // No post-install executable launch is performed by the bootstrapper.
    Ok(())
}
