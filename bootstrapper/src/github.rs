use serde::Deserialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

pub async fn get_latest_obs_build_info() -> anyhow::Result<(String, String, String)> {
    let url = "https://api.github.com/repos/libobs-rs/libobs-builds/releases/latest";
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "bootstrapper")
        .send()
        .await?;
    let release: Release = resp.json().await?;
    let version = release
        .tag_name
        .trim_start_matches("obs-build-")
        .to_string();
    let mut asset_url = String::new();
    let mut sha256_url = String::new();
    println!("Found assets in latest release:");
    for asset in &release.assets {
        println!("  {} -> {}", asset.name, asset.browser_download_url);
        if asset.name.ends_with("obs-build.7z") {
            asset_url = asset.browser_download_url.clone();
        } else if asset.name.ends_with("obs-build.sha256") {
            sha256_url = asset.browser_download_url.clone();
        }
    }
    if asset_url.is_empty() || sha256_url.is_empty() {
        anyhow::bail!(
            "Could not find obs-build.7z or obs-build.7z.sha256 in the latest release assets"
        );
    }
    Ok((version, asset_url, sha256_url))
}

pub async fn download_latest_obs_build(
    asset_url: &str,
    sha256: &str,
    out_path: &Path,
) -> anyhow::Result<()> {
    let resp = reqwest::get(asset_url).await?;
    let bytes = resp.bytes().await?;
    let mut file = File::create(out_path)?;
    file.write_all(&bytes)?;
    // Verify SHA256
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());
    if hash != sha256 {
        anyhow::bail!("SHA256 mismatch: expected {}, got {}", sha256, hash);
    }
    Ok(())
}
