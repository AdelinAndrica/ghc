// src/git_ops.rs
use anyhow::{Result, anyhow};

pub fn git_clone(url: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("clone")
        .arg(url)
        .status()?;

    if !status.success() {
        return Err(anyhow!(
            "git clone failed with exit code {:?}",
            status.code()
        ));
    }

    Ok(())
}
