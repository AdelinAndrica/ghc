// src/main.rs
use anyhow::{Context, Result, anyhow};

mod cli;
mod exit_codes;
mod git;
use clap::Parser;

// your existing modules
mod auth;
mod github;
mod models;
mod tui;

use cli::{Args, CloneProtocol, Cmd};
use exit_codes::ExitCode;

#[tokio::main]
async fn main() {
    let code = match run().await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::GithubApi // generic “app error” bucket; feel free to change
        }
    };

    std::process::exit(code.as_i32());
}

async fn run() -> Result<ExitCode> {
    let args = Args::parse();
    let proto = CloneProtocol::from_flags(args.ssh, args.https);

    match args.cmd {
        Some(Cmd::Login) => {
            auth::login_device_flow().await?;
            println!("Login successful.");
            return Ok(ExitCode::Ok);
        }
        Some(Cmd::AuthStatus) => {
            auth::print_auth_status()?;
            return Ok(ExitCode::Ok);
        }
        Some(Cmd::Logout) => {
            auth::logout()?;
            println!("Logged out (local token removed).");
            return Ok(ExitCode::Ok);
        }
        Some(Cmd::Clone { repo }) => {
            // Script-friendly path:
            // - Validate repo spec
            // - Clone
            // - Return exit codes appropriate for scripting
            let owner_repo = git::normalize_repo_spec(&repo).map_err(|e| anyhow!(e))?;

            match git::git_clone(&owner_repo, proto) {
                Ok(()) => Ok(ExitCode::Ok),
                Err(code) => Ok(code),
            }
        }
        None => {
            // Interactive flow (unchanged except protocol flags):
            let token = auth::resolve_token()
                .context("Not authenticated. Run `ghc login` or set GITHUB_TOKEN.")?;

            github::validate_token(&token)
                .await
                .map_err(|e| anyhow!("{e}"))
                .context("Token validation failed")?;

            let repos = github::fetch_repos(&token, args.owned).await?;

            // load last query (v0.0.6 feature) from your auth/settings module
            let initial_query = auth::load_last_query().unwrap_or_default();
            let out = tui::run_tui(repos, args.archived, args.sort, initial_query)?;

            // persist last query
            let _ = auth::save_last_query(&out.last_query);

            if let Some(repo) = out.selected {
                // Your Repo has ssh_url/clone_url in models; we keep that for interactive mode:
                let url = match proto {
                    CloneProtocol::Ssh => repo.ssh_url,
                    CloneProtocol::Https => repo.clone_url,
                };

                // Use the improved clone implementation too:
                // Convert URL to owner/repo if possible, else just run `git clone <url>` with the same helpers.
                // Minimal approach: just call git clone with URL directly:
                match git_clone_url(&url) {
                    Ok(()) => Ok(ExitCode::Ok),
                    Err(code) => Ok(code),
                }
            } else {
                Ok(ExitCode::Ok)
            }
        }
    }
}

/// Same improved errors/stdout-stderr behavior, but for a full URL (interactive mode).
fn git_clone_url(url: &str) -> std::result::Result<(), ExitCode> {
    use std::io::{self, Write};
    use std::process::{Command, Stdio};

    let out = Command::new("git")
        .arg("clone")
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let out = match out {
        Ok(o) => o,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            eprintln!("Error: `git` was not found on PATH. Install Git and try again.");
            return Err(ExitCode::GitNotFound);
        }
        Err(e) => {
            eprintln!("Error: failed to spawn `git clone`: {e}");
            return Err(ExitCode::GitCloneFailed);
        }
    };

    if !out.stdout.is_empty() {
        let _ = io::stdout().write_all(&out.stdout);
        let _ = io::stdout().flush();
    }
    if !out.stderr.is_empty() {
        let _ = io::stderr().write_all(&out.stderr);
        let _ = io::stderr().flush();
    }

    if out.status.success() {
        Ok(())
    } else {
        let code = out.status.code().unwrap_or(-1);
        eprintln!();
        eprintln!("Error: `git clone` failed (exit code {code}).");
        Err(ExitCode::GitCloneFailed)
    }
}
