use crate::{constants::CLIENT_FILE_VAULT_MODIFIED, current::current_local_path};

pub async fn check_vault_modified() -> bool {
    let Some(current_dir) = current_local_path() else {
        return false;
    };

    let record_file = current_dir.join(CLIENT_FILE_VAULT_MODIFIED);
    if !record_file.exists() {
        return false;
    }

    let Ok(contents) = tokio::fs::read_to_string(&record_file).await else {
        return false;
    };

    matches!(contents.trim().to_lowercase().as_str(), "true")
}

pub async fn sign_vault_modified(modified: bool) {
    let Some(current_dir) = current_local_path() else {
        return;
    };

    let record_file = current_dir.join(CLIENT_FILE_VAULT_MODIFIED);

    let contents = if modified { "true" } else { "false" };

    let _ = tokio::fs::write(&record_file, contents).await;
}
