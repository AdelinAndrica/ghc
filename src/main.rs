// src/main.rs
mod auth;
mod cli;
mod git_ops;
mod github;
mod models;
mod state;
mod tui;

use anyhow::{Result, anyhow};
use clap::Parser;
use cli::{Args, Cmd};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Some(Cmd::Login) => {
            auth::login_device_flow().await?;
            println!("Login successful.");
            return Ok(());
        }
        Some(Cmd::AuthStatus) => {
            auth::print_auth_status()?;
            return Ok(());
        }
        Some(Cmd::Logout) => {
            auth::logout()?;
            println!("Logged out (local token removed).");
            return Ok(());
        }
        None => {}
    }

    let token = auth::resolve_token()?;
    github::validate_token(&token).await?;

    let repos = github::fetch_repos(&token, args.owned).await?;

    // v0.0.6: remember last query between runs
    let initial_query = state::load_last_query().unwrap_or_default();

    let tui_out = tui::run_tui(repos, args.archived, args.sort, initial_query)?;

    // persist query even if user cancels
    state::save_last_query(&tui_out.last_query).ok();

    if let Some(repo) = tui_out.selected {
        let url = if args.ssh {
            repo.ssh_url
        } else {
            repo.clone_url
        };
        git_ops::git_clone(&url).map_err(|e| anyhow!("{e}"))?;
    }

    Ok(())
}
