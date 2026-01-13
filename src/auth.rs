// src/auth.rs
use crate::models::{DeviceCodeResponse, StoredAuth, TokenErrResponse, TokenOkResponse};
use anyhow::{Context, Result, anyhow};

use directories::ProjectDirs;
use reqwest::header::{ACCEPT, HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tokio::time::{Duration, sleep};

const DEFAULT_GITHUB_CLIENT_ID: &str = "Ov23liadwk2DQHKgANja";

fn github_client_id() -> String {
    std::env::var("GHC_GITHUB_CLIENT_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_GITHUB_CLIENT_ID.to_string())
}

fn auth_path() -> anyhow::Result<PathBuf> {
    let proj = ProjectDirs::from("com", "ghc", "ghc")
        .ok_or_else(|| anyhow!("Could not determine config directory"))?;
    let dir = proj.config_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.join("auth.json"))
}

fn save_token(token: &str) -> anyhow::Result<()> {
    let path = auth_path()?;
    let data = StoredAuth {
        github_token: token.to_string(),
    };
    fs::write(&path, serde_json::to_vec_pretty(&data)?)?;
    Ok(())
}

fn load_token() -> anyhow::Result<Option<String>> {
    let path = auth_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    let data: StoredAuth = serde_json::from_slice(&bytes)?;
    Ok(Some(data.github_token))
}

pub fn logout() -> anyhow::Result<()> {
    let path = auth_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn print_auth_status() -> anyhow::Result<()> {
    if std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        println!("Auth: GITHUB_TOKEN (env) is set.");
        return Ok(());
    }
    if load_token()?.is_some() {
        let path = auth_path()?;
        println!("Auth: stored token at {}", path.display());
        return Ok(());
    }
    println!("Auth: not logged in. Run `ghc login` or set GITHUB_TOKEN.");
    Ok(())
}

pub fn resolve_token() -> anyhow::Result<String> {
    if let Ok(t) = std::env::var("GITHUB_TOKEN") {
        let t = t.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }

    if let Some(t) = load_token()? {
        let t = t.trim().to_string();
        if !t.is_empty() {
            return Ok(t);
        }
    }

    Err(anyhow!(
        "Not authenticated. Run `ghc login` or set GITHUB_TOKEN."
    ))
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Settings {
    last_query: String,
}

fn settings_path() -> Result<PathBuf> {
    let proj = ProjectDirs::from("com", "ghc", "ghc")
        .ok_or_else(|| anyhow!("Could not determine config directory"))?;
    let dir = proj.config_dir();
    fs::create_dir_all(dir)?;
    Ok(dir.join("settings.json"))
}

pub fn load_last_query() -> Result<String> {
    let p = settings_path()?;
    if !p.exists() {
        return Ok(String::new());
    }
    let bytes = fs::read(p)?;
    let s: Settings = serde_json::from_slice(&bytes)?;
    Ok(s.last_query)
}

pub fn save_last_query(q: &str) -> Result<()> {
    let p = settings_path()?;
    let s = Settings {
        last_query: q.to_string(),
    };
    fs::write(p, serde_json::to_vec_pretty(&s)?)?;
    Ok(())
}

pub async fn login_device_flow() -> anyhow::Result<()> {
    let client_id = github_client_id();

    if client_id.trim().is_empty() || client_id == "PASTE_YOUR_CLIENT_ID_HERE" {
        return Err(anyhow!(
            "OAuth client id is not configured. Set DEFAULT_GITHUB_CLIENT_ID in source \
or set GHC_GITHUB_CLIENT_ID for this session."
        ));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("Failed to initialize HTTP client")?;

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("ghc"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    let dc: DeviceCodeResponse = client
        .post("https://github.com/login/device/code")
        .headers(headers.clone())
        .form(&[
            ("client_id", client_id.as_str()),
            ("scope", "repo read:org"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    println!("Open this URL in your browser:");
    println!("{}", dc.verification_uri);
    println!("(If that doesn't open, use: https://github.com/login/device)");
    println!();
    println!("And enter this code:");
    println!("{}", dc.user_code);
    println!();

    let interval = dc.interval.unwrap_or(5);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(dc.expires_in);

    loop {
        if std::time::Instant::now() > deadline {
            return Err(anyhow!("Login timed out. Please run `ghc login` again."));
        }

        let resp = client
            .post("https://github.com/login/oauth/access_token")
            .headers(headers.clone())
            .form(&[
                ("client_id", client_id.as_str()),
                ("device_code", dc.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?;

        if let Ok(ok) = serde_json::from_slice::<TokenOkResponse>(&resp) {
            save_token(&ok.access_token)?;
            return Ok(());
        }

        let err = serde_json::from_slice::<TokenErrResponse>(&resp).unwrap_or(TokenErrResponse {
            error: "unknown_error".to_string(),
            error_description: None,
        });

        match err.error.as_str() {
            "authorization_pending" => sleep(Duration::from_secs(interval)).await,
            "slow_down" => sleep(Duration::from_secs(interval + 5)).await,
            "access_denied" => return Err(anyhow!("Access denied in browser.")),
            "expired_token" => {
                return Err(anyhow!(
                    "Device code expired. Please run `ghc login` again."
                ));
            }
            _ => {
                return Err(anyhow!(
                    "Login failed: {}{}",
                    err.error,
                    err.error_description
                        .as_ref()
                        .map(|d| format!(" ({})", d))
                        .unwrap_or_default()
                ));
            }
        }
    }
}
