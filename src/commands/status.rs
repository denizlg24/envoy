use crate::utils::{
    manifest::{load_manifest, read_applied},
    project_config::load_project_config,
    ui::{print_header, print_info, print_kv, print_success, print_warn},
};
use console::style;
use std::path::Path;

pub fn status(passphrase: &str) -> anyhow::Result<()> {
    let project = load_project_config()?;

    print_header("Envoy Status");
    print_kv("Project:", &project.project_id);

    let latest = match std::fs::read_to_string(".envoy/latest") {
        Ok(v) => v.trim().to_string(),
        Err(_) => {
            print_info("Manifest: none");
            println!();
            print_warn("State: EMPTY");
            print_info(&format!("Run {}", style("`envy push`").cyan()));
            return Ok(());
        }
    };

    let applied = read_applied();

    print_kv("Manifest:", &latest[..12]);

    if let Some(ref applied) = applied {
        print_kv("Applied:", &applied[..12]);
    }

    let manifest_path = format!(".envoy/cache/{}.blob", latest);

    if !Path::new(&manifest_path).exists() {
        println!();
        print_warn("Manifest blob missing locally");
        print_warn("State: OUT OF SYNC");
        print_info(&format!("Run {}", style("`envy pull`").cyan()));
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

    println!();

    if !has_all_blobs {
        print_warn("State: OUT OF SYNC (missing data)");
        print_info(&format!("Run {}", style("`envy pull`").cyan()));
    } else if !is_applied {
        print_warn("State: OUT OF SYNC (not applied)");
        print_info(&format!("Run {}", style("`envy pull`").cyan()));
    } else {
        print_success("State: UP TO DATE");
    }

    Ok(())
}
