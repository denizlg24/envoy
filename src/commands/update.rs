use anyhow::{Result, anyhow};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::path::{Path, PathBuf};

const REPO: &str = "denizlg24/envoy";
const USER_AGENT: &str = "envy-cli";

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

pub async fn update() -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message("Checking for updates...");

    let release = fetch_latest_release().await?;
    let latest = release.tag_name.trim_start_matches('v');

    spinner.finish_and_clear();

    if latest == CURRENT_VERSION {
        println!(
            "{} {}",
            style("✓").green().bold(),
            style(format!("Already up to date (v{})", CURRENT_VERSION)).green()
        );
        return Ok(());
    }

    println!(
        "\n{} {}",
        style("→").cyan().bold(),
        style("Update Available").bold()
    );
    println!("  {} v{}", style("Current:").dim(), CURRENT_VERSION);
    println!(
        "  {} v{}",
        style("Latest: ").dim(),
        style(latest).yellow().bold()
    );

    let asset_name = platform_asset_name()?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| anyhow!("No release asset for this platform"))?;

    println!();
    let extracted_binary = download_and_extract(&asset.browser_download_url).await?;

    eprintln!("[DEBUG] Extracted binary path: {:?}", extracted_binary);
    eprintln!(
        "[DEBUG] Extracted binary exists before replace_self: {}",
        extracted_binary.exists()
    );

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message("Applying update...");
    replace_self(&extracted_binary)?;
    spinner.finish_and_clear();

    // Clean up temporary binary
    let _ = std::fs::remove_file(&extracted_binary);

    println!(
        "{} {}",
        style("✓").green().bold(),
        style("Update complete!").green()
    );
    println!(
        "  {} {}",
        style("ℹ").cyan(),
        style("Restart envoy to use the new version").dim()
    );
    Ok(())
}

async fn fetch_latest_release() -> Result<Release> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);

    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json::<Release>()
        .await?;

    Ok(res)
}

fn platform_asset_name() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("envoy-aarch64-apple-darwin.tar.gz"),
        ("macos", "x86_64") => Ok("envoy-x86_64-apple-darwin.tar.gz"),
        ("linux", "x86_64") => Ok("envoy-x86_64-unknown-linux-musl.tar.gz"),
        ("windows", "x86_64") => Ok("envoy-x86_64-pc-windows-msvc.zip"),
        _ => Err(anyhow!("Unsupported platform")),
    }
}

async fn download_and_extract(url: &str) -> Result<PathBuf> {
    let tmp = tempfile::tempdir()?;
    let archive_path = tmp.path().join("archive");

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));
    spinner.set_message("Downloading update...");

    let bytes = reqwest::get(url).await?.bytes().await?;
    spinner.set_message("Extracting archive...");
    tokio::fs::write(&archive_path, &bytes).await?;

    let url_owned = url.to_string();
    let archive_clone = archive_path.clone();
    let tmp_path = tmp.path().to_path_buf();

    let binary_in_tmp = tokio::task::spawn_blocking(move || -> Result<PathBuf> {
        if url_owned.ends_with(".tar.gz") {
            extract_tar_gz(&archive_clone, &tmp_path)?;
        } else if url_owned.ends_with(".zip") {
            extract_zip(&archive_clone, &tmp_path)?;
        } else {
            return Err(anyhow!("Unknown archive format"));
        }
        let binary = find_binary(&tmp_path)?;
        eprintln!("[DEBUG] Found binary in archive: {:?}", binary);
        eprintln!("[DEBUG] Binary exists: {}", binary.exists());
        Ok(binary)
    })
    .await??;

    eprintln!("[DEBUG] Binary in tmp path: {:?}", binary_in_tmp);
    eprintln!("[DEBUG] Binary in tmp exists: {}", binary_in_tmp.exists());

    let persistent_path = std::env::temp_dir().join(format!("envy-update-{}", std::process::id()));
    eprintln!("[DEBUG] Copying to persistent path: {:?}", persistent_path);
    tokio::fs::copy(&binary_in_tmp, &persistent_path).await?;
    eprintln!(
        "[DEBUG] Copy complete, persistent file exists: {}",
        persistent_path.exists()
    );

    spinner.finish_and_clear();

    println!(
        "{} {}",
        style("✓").green().bold(),
        style("Downloaded and extracted").green()
    );

    Ok(persistent_path)
}

fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let file = std::fs::File::open(archive)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive.unpack(dest)?;
    Ok(())
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<()> {
    use zip::ZipArchive;

    let file = std::fs::File::open(archive)?;
    let mut zip = ZipArchive::new(file)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let out = dest.join(entry.name());

        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }

    Ok(())
}

fn find_binary(dir: &Path) -> Result<PathBuf> {
    let exe_name = if cfg!(windows) { "envy.exe" } else { "envy" };

    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_name() == exe_name {
            return Ok(entry.path().to_path_buf());
        }
    }

    Err(anyhow!("Binary not found in archive"))
}

fn replace_self(new_binary: &Path) -> Result<()> {
    eprintln!("[DEBUG] replace_self called with: {:?}", new_binary);
    eprintln!("[DEBUG] new_binary exists: {}", new_binary.exists());

    let current = std::env::current_exe()?;
    eprintln!("[DEBUG] current exe: {:?}", current);
    let backup = current.with_extension("old");
    eprintln!("[DEBUG] backup path: {:?}", backup);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(new_binary)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(new_binary, perms)?;
    }

    if current.exists() {
        let _ = std::fs::rename(&current, &backup);
    }

    std::fs::copy(new_binary, &current)?;

    let _ = std::fs::remove_file(backup);
    Ok(())
}
