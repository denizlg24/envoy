use anyhow::Ok;
use console::style;

use crate::utils::{
    manifest::{load_manifest, save_manifest},
    paths::normalize_path,
    ui::{print_error, print_info, print_success},
};

pub fn remove_file(path: &str) -> anyhow::Result<()> {
    let mut manifest = load_manifest()?;
    let normalized = normalize_path(path);

    if !manifest.files.contains_key(&normalized) {
        print_error(&format!("File '{}' is not tracked.", path));
        return Ok(());
    }

    manifest.files.remove(&normalized);
    save_manifest(&manifest)?;

    print_success(&format!("Removed '{}'.", normalized));
    print_info(&format!(
        "Run {} to record this change.",
        style("`envy commit -m \"message\"`").cyan()
    ));
    Ok(())
}
