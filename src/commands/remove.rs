use anyhow::Ok;
use console::style;

use crate::utils::{
    manifest::{load_manifest, save_manifest},
    ui::{print_error, print_info, print_success},
};

pub fn remove_file(path: &str) -> anyhow::Result<()> {
    let mut manifest = load_manifest()?;
    if !manifest.files.contains_key(path) {
        print_error(&format!("File '{}' is not tracked.", path));
        return Ok(());
    }
    manifest.files.remove(path);
    save_manifest(&manifest)?;

    print_success(&format!("Removed '{}'.", path));
    print_info(&format!(
        "Run {} to record this change.",
        style("`envy commit -m \"message\"`").cyan()
    ));
    Ok(())
}
