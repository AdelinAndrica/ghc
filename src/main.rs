use anyhow::{Context, Result, anyhow};
use crossterm::event::KeyEventKind;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};
use std::{io, process::Command};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

use tokio::time::{Duration, sleep};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "ghc", about = "Interactive GitHub repo picker + clone")]
struct Args {
    #[command(subcommand)]
    cmd: Option<Cmd>,

    /// Clone using SSH instead of HTTPS
    #[arg(long)]
    ssh: bool,

    /// Only show repos owned by the authenticated user
    #[arg(long)]
    owned: bool,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Authenticate with GitHub using device flow and store token locally
    Login,
    /// Print the currently stored token source (debug)
    AuthStatus,
    /// Remove locally stored token
    Logout,
}

#[derive(Debug, Clone, Deserialize)]
struct Repo {
    full_name: String, // "owner/name"
    description: Option<String>,
    ssh_url: String,
    clone_url: String,
    archived: bool,
    fork: bool,
    private: bool,
}

const DEFAULT_GITHUB_CLIENT_ID: &str = "Ov23liadwk2DQHKgANja";

fn github_client_id() -> String {
    std::env::var("GHC_GITHUB_CLIENT_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_GITHUB_CLIENT_ID.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.cmd {
        Some(Cmd::Login) => {
            login_device_flow().await?;
            println!("Login successful.");
            return Ok(());
        }
        Some(Cmd::AuthStatus) => {
            print_auth_status()?;
            return Ok(());
        }
        Some(Cmd::Logout) => {
            logout()?;
            println!("Logged out (local token removed).");
            return Ok(());
        }
        None => {}
    }

    let token = resolve_token()?; // updated to no longer require `gh`
    let repos = fetch_repos(&token, args.owned).await?;
    let selection = run_tui(repos)?;
    if let Some(repo) = selection {
        let url = if args.ssh {
            repo.ssh_url
        } else {
            repo.clone_url
        };
        let status = std::process::Command::new("git")
            .arg("clone")
            .arg(url)
            .status()?;
        if !status.success() {
            return Err(anyhow::anyhow!(
                "git clone failed with exit code {:?}",
                status.code()
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredAuth {
    github_token: String,
}

fn auth_path() -> anyhow::Result<PathBuf> {
    let proj = ProjectDirs::from("com", "ghc", "ghc")
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?;
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

fn logout() -> anyhow::Result<()> {
    let path = auth_path()?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn print_auth_status() -> anyhow::Result<()> {
    if std::env::var("GITHUB_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
    {
        println!("Auth: GITHUB_TOKEN (env) is set.");
        return Ok(());
    }
    if let Some(_) = load_token()? {
        let path = auth_path()?;
        println!("Auth: stored token at {}", path.display());
        return Ok(());
    }
    println!("Auth: not logged in. Run `ghc login` or set GITHUB_TOKEN.");
    Ok(())
}

fn resolve_token() -> anyhow::Result<String> {
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

    Err(anyhow::anyhow!(
        "Not authenticated. Run `ghc login` or set GITHUB_TOKEN."
    ))
}

async fn fetch_repos(token: &str, owned_only: bool) -> Result<Vec<Repo>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))?,
    );
    headers.insert(USER_AGENT, HeaderValue::from_static("ghc-cli"));

    let mut page = 1u32;
    let mut all: Vec<Repo> = Vec::new();

    loop {
        let mut url =
            format!("https://api.github.com/user/repos?per_page=100&page={page}&sort=updated");
        // owned_only: use affiliation=owner
        if owned_only {
            url.push_str("&affiliation=owner");
        } else {
            // includes owner, org member, collaborations
            url.push_str("&affiliation=owner,collaborator,organization_member");
        }

        let batch: Vec<Repo> = client
            .get(&url)
            .headers(headers.clone())
            .send()
            .await
            .context("Failed to call GitHub API")?
            .error_for_status()
            .context("GitHub API returned an error")?
            .json()
            .await
            .context("Failed to parse GitHub API response")?;

        if batch.is_empty() {
            break;
        }

        all.extend(batch);
        page += 1;
    }

    Ok(all)
}

fn run_tui(repos: Vec<Repo>) -> Result<Option<Repo>> {
    enable_raw_mode().context("enable_raw_mode failed")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("EnterAlternateScreen failed")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Terminal init failed")?;
    terminal.clear().ok();

    let mut selected_idx: usize = 0;
    let mut query = String::new();
    let mut result: Option<Repo> = None;

    // Recompute when query changes
    let mut filtered_indices: Vec<usize> = (0..repos.len()).collect();

    let mut needs_redraw = true;

    loop {
        if needs_redraw {
            // recompute filtered list
            filtered_indices = filter_indices(&repos, &query);

            if filtered_indices.is_empty() {
                selected_idx = 0;
            } else if selected_idx >= filtered_indices.len() {
                selected_idx = filtered_indices.len() - 1;
            }

            terminal.draw(|f| {
                let area = f.area();

                // Layout: top search, middle split list+details, bottom help
                let outer = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(1),
                        Constraint::Length(2),
                    ])
                    .split(area);

                let middle = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .split(outer[1]);

                // Search bar
                let search = Paragraph::new(format!("Search: {}", query))
                    .block(Block::default().borders(Borders::ALL).title("ghc repo"));
                f.render_widget(search, outer[0]);

                // List: single-line items so you can see more repos
                let items: Vec<ListItem> = filtered_indices
                    .iter()
                    .map(|&i| {
                        let r = &repos[i];
                        // compact meta markers
                        let mut meta = String::new();
                        if r.private {
                            meta.push_str(" 🔒");
                        }
                        if r.fork {
                            meta.push_str(" ⑂");
                        }
                        if r.archived {
                            meta.push_str(" 📦");
                        }

                        ListItem::new(Line::from(format!("{}{}", r.full_name, meta)))
                    })
                    .collect();

                let mut state = ratatui::widgets::ListState::default();
                if !filtered_indices.is_empty() {
                    state.select(Some(selected_idx));
                }

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(format!(
                        "Repositories ({}/{})",
                        filtered_indices.len(),
                        repos.len()
                    )))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("➤ ");

                f.render_stateful_widget(list, middle[0], &mut state);

                // Details pane
                let details_text = if filtered_indices.is_empty() {
                    "No matches.".to_string()
                } else {
                    let r = &repos[filtered_indices[selected_idx]];
                    let desc = r
                        .description
                        .clone()
                        .unwrap_or_else(|| "(no description)".to_string());
                    format!(
                        "{}\n\n{}\n\nprivate: {}\nfork: {}\narchived: {}",
                        r.full_name, desc, r.private, r.fork, r.archived
                    )
                };

                let details = Paragraph::new(details_text)
                    .block(Block::default().borders(Borders::ALL).title("Details"));
                f.render_widget(details, middle[1]);

                // Help
                let help = Paragraph::new(
                    "↑/↓ move • type to filter • Backspace • Enter clone • Esc quit • Ctrl+C quit",
                )
                .block(Block::default().borders(Borders::ALL));
                f.render_widget(help, outer[2]);
            })?;

            needs_redraw = false;
        }

        // Block waiting for an event (no constant redraw = no flicker)
        match event::read()? {
            Event::Key(key) => {
                // IMPORTANT: ignore release/repeat to stop double-typing
                if key.kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }

                if key.code == KeyCode::Esc
                    || (key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL))
                {
                    break;
                }

                match key.code {
                    KeyCode::Up => {
                        if selected_idx > 0 {
                            selected_idx -= 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Down => {
                        if !filtered_indices.is_empty() && selected_idx + 1 < filtered_indices.len()
                        {
                            selected_idx += 1;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Enter => {
                        if !filtered_indices.is_empty() {
                            result = Some(repos[filtered_indices[selected_idx]].clone());
                        }
                        break;
                    }
                    KeyCode::Backspace => {
                        if !query.is_empty() {
                            query.pop();
                            selected_idx = 0;
                            needs_redraw = true;
                        }
                    }
                    KeyCode::Char(ch) => {
                        if !key.modifiers.contains(KeyModifiers::CONTROL)
                            && !key.modifiers.contains(KeyModifiers::ALT)
                        {
                            query.push(ch);
                            selected_idx = 0;
                            needs_redraw = true;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // cleanup terminal
    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    Ok(result)
}

fn filter_indices(repos: &[Repo], query: &str) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return (0..repos.len()).collect();
    }

    repos
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let name = r.full_name.to_lowercase();
            let desc = r.description.clone().unwrap_or_default().to_lowercase();
            if name.contains(&q) || desc.contains(&q) {
                Some(i)
            } else {
                None
            }
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TokenOkResponse {
    access_token: String,
    token_type: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct TokenErrResponse {
    error: String, // authorization_pending, slow_down, expired_token, access_denied
    error_description: Option<String>,
}

async fn login_device_flow() -> anyhow::Result<()> {
    let client_id = github_client_id();

    if client_id.trim().is_empty() || client_id == "PASTE_YOUR_CLIENT_ID_HERE" {
        return Err(anyhow::anyhow!(
            "OAuth client id is not configured. Set DEFAULT_GITHUB_CLIENT_ID in source \
or set GHC_GITHUB_CLIENT_ID for this session."
        ));
    }

    let client = reqwest::Client::new();

    // 1) Request device code
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("ghc"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

    // Scopes: repo (private repos), read:org (org repos)
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

    // 2) Poll for token
    let interval = dc.interval.unwrap_or(5);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(dc.expires_in);

    loop {
        if std::time::Instant::now() > deadline {
            return Err(anyhow::anyhow!(
                "Login timed out. Please run `ghc login` again."
            ));
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

        // Response is JSON because we set Accept: application/json
        // It can be either ok or error.
        if let Ok(ok) = serde_json::from_slice::<TokenOkResponse>(&resp) {
            save_token(&ok.access_token)?;
            return Ok(());
        }

        let err = serde_json::from_slice::<TokenErrResponse>(&resp).unwrap_or(TokenErrResponse {
            error: "unknown_error".to_string(),
            error_description: None,
        });

        match err.error.as_str() {
            "authorization_pending" => {
                sleep(Duration::from_secs(interval)).await;
            }
            "slow_down" => {
                sleep(Duration::from_secs(interval + 5)).await;
            }
            "access_denied" => {
                return Err(anyhow::anyhow!("Access denied in browser."));
            }
            "expired_token" => {
                return Err(anyhow::anyhow!(
                    "Device code expired. Please run `ghc login` again."
                ));
            }
            _ => {
                return Err(anyhow::anyhow!(
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
