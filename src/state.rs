// src/state.rs
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Default, Serialize, Deserialize)]
struct UiState {
    last_query: String,
}

fn state_path() -> Result<PathBuf> {
    let proj = ProjectDirs::from("com", "ghc", "ghc")
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
    let dir = proj.config_dir();
    fs::create_dir_all(dir).context("Failed to create config directory")?;
    Ok(dir.join("ui_state.json"))
}

pub fn load_last_query() -> Result<String> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(String::new());
    }
    let bytes = fs::read(&path).context("Failed to read ui_state.json")?;
    let s: UiState = serde_json::from_slice(&bytes).context("Failed to parse ui_state.json")?;
    Ok(s.last_query)
}

pub fn save_last_query(query: &str) -> Result<()> {
    let path = state_path()?;
    let s = UiState {
        last_query: query.to_string(),
    };
    fs::write(&path, serde_json::to_vec_pretty(&s)?).context("Failed to write ui_state.json")?;
    Ok(())
}
