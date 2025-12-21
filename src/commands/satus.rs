use crate::utils::{manifest::load_manifest, project_config::load_project_config};
use console::style;
use std::path::Path;

pub fn status(passphrase: Option<&str>) -> anyhow::Result<()> {
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

    println!(
        "{} {}",
        style("📄").cyan(),
        style(format!("Manifest: {}", &latest[..12])).dim()
    );

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

    // Optional: allow status without passphrase
    let manifest = match passphrase {
        Some(p) => load_manifest(p)?,
        None => {
            println!(
                "\n{}",
                style("⚠ Cannot inspect files without passphrase").yellow()
            );
            println!("{} {}", style("○").dim(), style("State: UNKNOWN").dim());
            return Ok(());
        }
    };

    let mut present = 0;
    let mut missing = 0;

    for hash in manifest.files.values() {
        let path = Path::new(".envoy/cache").join(format!("{}.blob", hash));
        if path.exists() {
            present += 1;
        } else {
            missing += 1;
        }
    }

    println!("\n{}", style("Blobs:").bold());
    println!(
        "  {} {}",
        style("✓").green(),
        style(format!("present: {}", present)).green()
    );
    if missing > 0 {
        println!(
            "  {} {}",
            style("✗").red(),
            style(format!("missing: {}", missing)).red()
        );
    }

    if missing == 0 {
        println!(
            "\n{} {}",
            style("●").green(),
            style("State: UP TO DATE").green().bold()
        );
    } else {
        println!(
            "\n{} {}",
            style("○").yellow(),
            style("State: OUT OF SYNC").yellow().bold()
        );
        println!(
            "{} run {}",
            style("💡").yellow(),
            style("`envy pull`").cyan()
        );
    }

    Ok(())
}
