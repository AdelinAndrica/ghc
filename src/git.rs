// src/git.rs
use anyhow::{Result, anyhow};
use std::io::{self, Write};
use std::process::{Command, Stdio};

use crate::cli::CloneProtocol;
use crate::exit_codes::ExitCode;

/// Build a clone URL from `owner/repo` and protocol.
/// Accepts input like:
/// - owner/repo
/// - https://github.com/owner/repo(.git)
/// - git@github.com:owner/repo(.git)
pub fn normalize_repo_spec(input: &str) -> Result<String> {
    let s = input.trim();

    // Already a github URL -> convert to owner/repo
    if let Some(rest) = s.strip_prefix("https://github.com/") {
        return Ok(strip_git_suffix(rest).to_string());
    }
    if let Some(rest) = s.strip_prefix("git@github.com:") {
        return Ok(strip_git_suffix(rest).to_string());
    }

    // Plain owner/repo
    if is_owner_repo(s) {
        return Ok(s.to_string());
    }

    Err(anyhow!(
        "Invalid repo spec: \"{s}\". Expected \"owner/repo\" (or a GitHub URL)."
    ))
}

fn strip_git_suffix(s: &str) -> &str {
    s.strip_suffix(".git").unwrap_or(s).trim_matches('/')
}

fn is_owner_repo(s: &str) -> bool {
    let mut parts = s.split('/');
    let owner = parts.next().unwrap_or("");
    let repo = parts.next().unwrap_or("");
    owner.len() >= 1
        && repo.len() >= 1
        && parts.next().is_none()
        && !owner.contains(' ')
        && !repo.contains(' ')
}

pub fn build_clone_url(owner_repo: &str, proto: CloneProtocol) -> String {
    match proto {
        CloneProtocol::Https => format!("https://github.com/{owner_repo}.git"),
        CloneProtocol::Ssh => format!("git@github.com:{owner_repo}.git"),
    }
}

pub fn git_clone(owner_repo: &str, proto: CloneProtocol) -> std::result::Result<(), ExitCode> {
    let url = build_clone_url(owner_repo, proto);

    // Capture output so we can provide better errors, but re-emit it to keep stdout/stderr separation.
    let out = Command::new("git")
        .arg("clone")
        .arg(&url)
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

    // Re-emit outputs with proper streams
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

        // Add a couple of common hints without being noisy
        if url.starts_with("https://") {
            eprintln!(
                "Hint: for private repos, HTTPS may require credentials/token configured in Git."
            );
        } else {
            eprintln!("Hint: make sure your SSH key is added to GitHub and ssh-agent is running.");
        }

        Err(ExitCode::GitCloneFailed)
    }
}
