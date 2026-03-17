use clap::Args;
use std::path::{Path, PathBuf};

#[derive(Debug, Args)]
pub struct GetAppArgs {
    /// Git URL, tarball URL (.tar.gz), zip URL (.zip), or local archive path
    pub source: String,

    /// Override the app directory name (default: derived from source)
    #[arg(long)]
    pub app_name: Option<String>,
}

pub async fn run(args: GetAppArgs) -> anyhow::Result<()> {
    let source = &args.source;

    // Determine source type and extract app name
    let app_name = args
        .app_name
        .clone()
        .unwrap_or_else(|| derive_app_name(source));
    let target = Path::new("apps").join(&app_name);

    if target.exists() {
        anyhow::bail!("Directory '{}' already exists", target.display());
    }

    std::fs::create_dir_all("apps")?;

    if is_archive(source) {
        if is_url(source) {
            // Download and extract archive
            download_and_extract(source, &target).await?;
        } else {
            // Local archive file
            extract_local(Path::new(source), &target)?;
        }
    } else {
        // Treat as git URL
        git_clone(source, &target)?;
    }

    // Verify loom_app.toml
    let app_toml = find_app_toml(&target);
    match app_toml {
        Some(toml_path) => {
            // If loom_app.toml is nested (e.g. archive extracted with a root dir),
            // move contents up
            let toml_parent = toml_path.parent().unwrap();
            if toml_parent != target {
                let temp = target.with_extension("_tmp");
                std::fs::rename(toml_parent, &temp)?;
                // Remove the original target (now has leftover dirs)
                let _ = std::fs::remove_dir_all(&target);
                std::fs::rename(&temp, &target)?;
            }
            tracing::info!("App '{}' installed into apps/{}", app_name, app_name);
        }
        None => {
            tracing::warn!(
                "Warning: loom_app.toml not found in '{}'. This may not be a valid Loom app.",
                target.display()
            );
        }
    }

    Ok(())
}

fn derive_app_name(source: &str) -> String {
    let s = source.trim_end_matches('/');
    let last = s.rsplit('/').next().unwrap_or(s);
    last.trim_end_matches(".git")
        .trim_end_matches(".tar.gz")
        .trim_end_matches(".tgz")
        .trim_end_matches(".zip")
        .to_string()
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn is_archive(s: &str) -> bool {
    let lower = s.to_lowercase();
    lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".zip")
        || (is_url(s) && (lower.contains("/archive/") || lower.contains("/releases/")))
}

fn git_clone(url: &str, target: &Path) -> anyhow::Result<()> {
    tracing::info!("Cloning {} into {}", url, target.display());
    let status = std::process::Command::new("git")
        .args(["clone", "--depth", "1", url, &target.to_string_lossy()])
        .status()?;
    if !status.success() {
        anyhow::bail!("git clone failed");
    }
    Ok(())
}

async fn download_and_extract(url: &str, target: &Path) -> anyhow::Result<()> {
    tracing::info!("Downloading {}...", url);

    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", response.status());
    }

    let bytes = response.bytes().await?;
    let tmp = tempfile::NamedTempFile::new()?;
    std::fs::write(tmp.path(), &bytes)?;

    let lower = url.to_lowercase();
    if lower.ends_with(".zip") {
        extract_zip(tmp.path(), target)?;
    } else {
        extract_tar_gz(tmp.path(), target)?;
    }

    Ok(())
}

fn extract_local(path: &Path, target: &Path) -> anyhow::Result<()> {
    tracing::info!("Extracting {}...", path.display());
    let lower = path.to_string_lossy().to_lowercase();
    if lower.ends_with(".zip") {
        extract_zip(path, target)?;
    } else {
        extract_tar_gz(path, target)?;
    }
    Ok(())
}

fn extract_tar_gz(archive_path: &Path, target: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);

    std::fs::create_dir_all(target)?;
    archive.unpack(target)?;

    Ok(())
}

fn extract_zip(archive_path: &Path, target: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    std::fs::create_dir_all(target)?;
    archive.extract(target)?;

    Ok(())
}

/// Find loom_app.toml — might be at the root or one level deep
/// (archives often have a single top-level directory)
fn find_app_toml(dir: &Path) -> Option<PathBuf> {
    // Check root
    let root = dir.join("loom_app.toml");
    if root.exists() {
        return Some(root);
    }

    // Check one level deep
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let nested = path.join("loom_app.toml");
                if nested.exists() {
                    return Some(nested);
                }
            }
        }
    }

    None
}
