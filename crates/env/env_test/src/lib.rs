use std::{
    env::{current_dir, set_current_dir},
    path::PathBuf,
};

use tokio::fs;

#[cfg(test)]
pub mod test_vault_setup_and_member_register;

#[cfg(test)]
pub mod test_virtual_file_creation_and_update;

pub async fn get_test_dir(area: &str) -> Result<PathBuf, std::io::Error> {
    let dir = current_dir()?.join(".temp").join("test").join(area);
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    } else {
        // Regenerate existing directory
        fs::remove_dir_all(&dir).await?;
        fs::create_dir_all(&dir).await?;
    }
    Ok(dir)
}
