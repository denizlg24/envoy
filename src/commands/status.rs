use crate::utils::{
    commit::{commits_ahead_of_remote, get_head_manifest_hash, read_head, read_remote_head},
    config::load_token,
    manifest::{
        compute_manifest_content_hash, get_current_manifest_hash, load_manifest,
        load_manifest_by_hash, read_applied,
    },
    project_config::{get_remote_url, load_project_config},
    storage::fetch_remote_head,
    ui::{print_header, print_info, print_kv, print_success, print_warn},
};
use console::style;
use std::path::Path;

pub async fn status() -> anyhow::Result<()> {
    let project = load_project_config()?;

    print_header("Envoy Status");
    print_kv("Project", &project.project_id);

    let current_manifest_hash = get_current_manifest_hash();
    let manifest = load_manifest()?;

    let local_head = read_head();
    let local_remote_head = read_remote_head();
    let head_manifest_hash = get_head_manifest_hash();

    let server_remote_head = {
        if let Ok(token) = load_token() {
            if let Ok(server) = get_remote_url(&project, None) {
                let client = reqwest::Client::new();
                fetch_remote_head(&client, &server, &token, &project.project_id)
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            }
        } else {
            None
        }
    };

    if let Some(ref hash) = current_manifest_hash {
        print_kv("Manifest", &hash[..12]);
    }

    print_kv("Files", &manifest.files.len().to_string());

    println!();

    if let Some(ref head) = local_head {
        print_kv("HEAD", &head[..12]);
    }

    if let Some(ref remote) = local_remote_head {
        print_kv("origin/HEAD", &remote[..12]);
    }

    let has_remote_commits =
        server_remote_head.is_some() && (local_head.is_none() || server_remote_head != local_head);
    let is_behind_remote =
        server_remote_head.is_some() && local_remote_head.as_ref() != server_remote_head.as_ref();

    let has_uncommitted_changes = {
        let current_content_hash = compute_manifest_content_hash(&manifest);
        match head_manifest_hash {
            Some(ref head_hash) => {
                match load_manifest_by_hash(head_hash) {
                    Ok(head_manifest) => {
                        let head_content_hash = compute_manifest_content_hash(&head_manifest);
                        current_content_hash != head_content_hash
                    }
                    Err(_) => true, // Can't load head manifest, assume changes exist
                }
            }
            None => {
                // No HEAD commit yet - uncommitted if there are files to commit
                !manifest.files.is_empty()
            }
        }
    };
    let commits_ahead = commits_ahead_of_remote().unwrap_or_default();
    let has_unpushed_commits = !commits_ahead.is_empty();

    let mut missing_blobs = 0;
    for hash in manifest.files.values() {
        let path = Path::new(".envoy/cache").join(format!("{}.blob", hash));
        if !path.exists() {
            missing_blobs += 1;
        }
    }

    // Check if files are applied locally
    let applied = read_applied();
    let is_applied = applied.as_ref() == current_manifest_hash.as_ref();

    println!();

    if has_unpushed_commits {
        print_info(&format!(
            "Your branch is {} commit(s) ahead of 'origin'.",
            commits_ahead.len()
        ));
    }
    if missing_blobs > 0 {
        print_warn("State: MISSING DATA");
        print_info(&format!(
            "{} file(s) missing locally. Run {}",
            missing_blobs,
            style("`envy pull`").cyan()
        ));
    } else if is_behind_remote || (has_remote_commits && local_head.is_none()) {
        print_warn("State: BEHIND REMOTE");
        print_info(&format!(
            "Remote has commits. Run {} to sync.",
            style("`envy pull`").cyan()
        ));
    } else if has_uncommitted_changes {
        print_warn("State: UNCOMMITTED CHANGES");
        if manifest.files.is_empty() && head_manifest_hash.is_some() {
            print_info(&format!(
                "All files removed. Run {} to record deletion.",
                style("`envy commit -m \"message\"`").cyan()
            ));
        } else if local_head.is_none() {
            print_info(&format!(
                "Run {} to create your first commit.",
                style("`envy commit -m \"message\"`").cyan()
            ));
        } else {
            print_info(&format!(
                "Run {} to commit your changes.",
                style("`envy commit -m \"message\"`").cyan()
            ));
        }
    } else if has_unpushed_commits {
        print_warn("State: UNPUSHED COMMITS");
        print_info(&format!(
            "Run {} to sync with remote.",
            style("`envy push`").cyan()
        ));
    } else if !is_applied && current_manifest_hash.is_some() {
        print_warn("State: NOT APPLIED");
        print_info(&format!(
            "Run {} to restore files locally.",
            style("`envy pull`").cyan()
        ));
    } else if local_head.is_none() && manifest.files.is_empty() && server_remote_head.is_none() {
        print_info("State: EMPTY");
        print_info(&format!(
            "Run {} to encrypt your first file.",
            style("`envy encrypt -i .env`").cyan()
        ));
    } else {
        print_success("State: UP TO DATE");
    }

    Ok(())
}
