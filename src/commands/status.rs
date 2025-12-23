use crate::utils::{
    manifest::{load_manifest, read_applied},
    project_config::load_project_config,
};
use console::style;
use std::path::Path;

pub fn status(passphrase: &str) -> anyhow::Result<()> {
    let project = load_project_config()?;

    println!("\n{}", style("Envoy Status").cyan().bold().underlined());
    println!(
        "\n{} {}",
        style("📦").cyan(),
        style(format!("Project: {}", project.project_id)).dim()
    );

    let latest = match std::fs::read_to_string(".envoy/latest") {
        Ok(v) => v.trim().to_string(),
        Err(_) => {
            println!("{} {}", style("📄").dim(), style("Manifest: none").dim());
            println!(
                "\n{} {}",
                style("○").yellow(),
                style("State: EMPTY").yellow().bold()
            );
            println!(
                "{} run {}",
                style("💡").yellow(),
                style("`envy push`").cyan()
            );
            return Ok(());
        }
    };

    let applied = read_applied();

    println!(
        "{} {}",
        style("📄").cyan(),
        style(format!("Manifest: {}", &latest[..12])).dim()
    );

    if let Some(ref applied) = applied {
        println!(
            "{} {}",
            style("🧩").cyan(),
            style(format!("Applied:  {}", &applied[..12])).dim()
        );
    }

    let manifest_path = format!(".envoy/cache/{}.blob", latest);

    if !Path::new(&manifest_path).exists() {
        println!("\n{}", style("⚠ Manifest blob missing locally").yellow());
        println!(
            "{} {}",
            style("○").yellow(),
            style("State: OUT OF SYNC").yellow().bold()
        );
        println!(
            "{} run {}",
            style("💡").yellow(),
            style("`envy pull`").cyan()
        );
        return Ok(());
    }

    let manifest = load_manifest(passphrase)?;

    let mut missing = 0;

    for hash in manifest.files.values() {
        let path = Path::new(".envoy/cache").join(format!("{}.blob", hash));
        if !path.exists() {
            missing += 1;
        }
    }

    let is_applied = applied.as_deref() == Some(&latest);
    let has_all_blobs = missing == 0;

    println!("\n{}", style("State:").bold());

    if !has_all_blobs {
        println!(
            "{} {}",
            style("○").yellow(),
            style("OUT OF SYNC (missing data)").yellow().bold()
        );
        println!(
            "{} run {}",
            style("💡").yellow(),
            style("`envy pull`").cyan()
        );
    } else if !is_applied {
        println!(
            "{} {}",
            style("○").yellow(),
            style("OUT OF SYNC (not applied)").yellow().bold()
        );
        println!(
            "{} run {}",
            style("💡").yellow(),
            style("`envy pull`").cyan()
        );
    } else {
        println!(
            "{} {}",
            style("●").green(),
            style("UP TO DATE").green().bold()
        );
    }

    Ok(())
}
