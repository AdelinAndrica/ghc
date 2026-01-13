// src/tui.rs
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use std::io;
use std::time::Duration;

use crate::cli::SortBy;
use crate::models::Repo;

pub struct TuiOutput {
    pub selected: Option<Repo>,
    pub last_query: String,
}

fn drain_pending_events() -> std::io::Result<()> {
    while event::poll(Duration::from_millis(0))? {
        let _ = event::read()?;
    }
    Ok(())
}

fn sort_label(sort: SortBy) -> &'static str {
    match sort {
        SortBy::Updated => "updated",
        SortBy::Name => "name",
        SortBy::Stars => "stars",
    }
}

fn sort_indices(repos: &[Repo], indices: &mut [usize], sort: SortBy) {
    match sort {
        // updated_at is ISO8601 => lexicographic compare is valid
        SortBy::Updated => indices.sort_by(|&a, &b| repos[b].updated_at.cmp(&repos[a].updated_at)),
        SortBy::Name => indices.sort_by(|&a, &b| repos[a].full_name.cmp(&repos[b].full_name)),
        SortBy::Stars => {
            indices.sort_by(|&a, &b| repos[b].stargazers_count.cmp(&repos[a].stargazers_count))
        }
    }
}

// v0.0.6: faster initial render for large lists
// - avoid doing expensive work repeatedly by:
//   - precomputing the “base list” once (archived-filtered + sorted)
//   - only re-filtering when query changes (still needed)
fn base_indices(repos: &[Repo], include_archived: bool, sort_by: SortBy) -> Vec<usize> {
    let mut idx: Vec<usize> = repos
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            if !include_archived && r.archived {
                None
            } else {
                Some(i)
            }
        })
        .collect();

    sort_indices(repos, &mut idx, sort_by);
    idx
}

fn filter_from_base(repos: &[Repo], base: &[usize], query: &str) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return base.to_vec();
    }

    base.iter()
        .copied()
        .filter(|&i| {
            let r = &repos[i];
            let name = r.full_name.to_lowercase();
            let desc = r.description.clone().unwrap_or_default().to_lowercase();
            name.contains(&q) || desc.contains(&q)
        })
        .collect()
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn help_text() -> Text<'static> {
    Text::from(vec![
        Line::from("Keyboard shortcuts"),
        Line::from(""),
        Line::from("  ↑/↓        move selection"),
        Line::from("  type       filter repositories"),
        Line::from("  Backspace  delete from query"),
        Line::from("  Enter      clone selected repo"),
        Line::from("  ?          toggle this help"),
        Line::from("  Esc        close help / quit"),
        Line::from("  Ctrl+C     quit"),
        Line::from(""),
        Line::from("Indicators"),
        Line::from("  🔒 private    ⑂ fork    📦 archived"),
    ])
}

pub fn run_tui(
    repos: Vec<Repo>,
    include_archived: bool,
    sort_by: SortBy,
    initial_query: String,
) -> Result<TuiOutput> {
    enable_raw_mode().context("enable_raw_mode failed")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("EnterAlternateScreen failed")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Terminal init failed")?;
    terminal.clear().ok();

    drain_pending_events().ok();

    let mut selected_idx: usize = 0;
    let mut query = initial_query;
    let mut result: Option<Repo> = None;

    // base list is precomputed once (archived-filtered + sorted)
    let base = base_indices(&repos, include_archived, sort_by);

    // filtered list derived from base
    let mut filtered_indices: Vec<usize> = Vec::new();

    let mut needs_redraw = true;

    // v0.0.6: help overlay
    let mut show_help = false;

    loop {
        if needs_redraw {
            filtered_indices = filter_from_base(&repos, &base, &query);

            if filtered_indices.is_empty() {
                selected_idx = 0;
            } else if selected_idx >= filtered_indices.len() {
                selected_idx = filtered_indices.len() - 1;
            }

            terminal.draw(|f| {
                let area = f.area();

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

                // List items (v0.0.6: consistent indicators across platforms)
                let items: Vec<ListItem> = filtered_indices
                    .iter()
                    .map(|&i| {
                        let r = &repos[i];

                        let mut meta = format!("  ★{}  forks:{}", r.stargazers_count, r.forks_count);

                        // indicators: always shown consistently
                        if r.private {
                            meta.push_str("  🔒");
                        }
                        if r.fork {
                            meta.push_str("  ⑂");
                        }
                        if r.archived {
                            meta.push_str("  📦");
                        }

                        ListItem::new(Line::from(format!("{}{}", r.full_name, meta)))
                    })
                    .collect();

                let mut state = ratatui::widgets::ListState::default();
                if !filtered_indices.is_empty() {
                    state.select(Some(selected_idx));
                }

                let list_title = format!(
                    "Repositories ({}/{}) • sort: {}{} • ? help",
                    filtered_indices.len(),
                    repos.len(),
                    sort_label(sort_by),
                    if include_archived { "" } else { " • archived hidden" }
                );

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title(list_title))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                    .highlight_symbol("➤ ");

                f.render_stateful_widget(list, middle[0], &mut state);

                // Details pane + empty states
                let details_text = if filtered_indices.is_empty() {
                    let q = query.trim();
                    if q.is_empty() {
                        if include_archived {
                            "No repositories found for this account.".to_string()
                        } else {
                            "No repositories to show.\n\nArchived repositories are hidden.\nRun with --archived to include them."
                                .to_string()
                        }
                    } else {
                        format!(
                            "No matches for: \"{}\"\n\nTry a shorter query, or run with --archived if you expect archived repos.",
                            q
                        )
                    }
                } else {
                    let r = &repos[filtered_indices[selected_idx]];
                    let desc = r
                        .description
                        .clone()
                        .unwrap_or_else(|| "(no description)".to_string());

                    format!(
                        "{}\n\n{}\n\n★ {}\nforks: {}\nupdated: {}\n\nprivate: {}\nfork: {}\narchived: {}",
                        r.full_name,
                        desc,
                        r.stargazers_count,
                        r.forks_count,
                        r.updated_at,
                        r.private,
                        r.fork,
                        r.archived
                    )
                };

                let details = Paragraph::new(details_text)
                    .block(Block::default().borders(Borders::ALL).title("Details"));
                f.render_widget(details, middle[1]);

                // Help
                let help = Paragraph::new(
                    "↑/↓ move • type to filter • Backspace • Enter clone • ? help • Esc quit • Ctrl+C quit",
                )
                .block(Block::default().borders(Borders::ALL));
                f.render_widget(help, outer[2]);

                // Help overlay (v0.0.6)
                if show_help {
                    let popup = centered_rect(70, 60, area);
                    f.render_widget(Clear, popup);
                    let p = Paragraph::new(help_text())
                        .block(Block::default().borders(Borders::ALL).title("Help"))
                        .alignment(Alignment::Left)
                        .wrap(Wrap { trim: false });
                    f.render_widget(p, popup);
                }
            })?;

            needs_redraw = false;
        }

        match event::read()? {
            Event::Key(key) => {
                // Keep your “double press” fix if you want it:
                // if key.kind != crossterm::event::KeyEventKind::Press { continue; }

                // Consistent quit keys across platforms
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }

                // Help overlay behavior
                if show_help {
                    match key.code {
                        KeyCode::Esc => {
                            show_help = false;
                            needs_redraw = true;
                        }
                        KeyCode::Char('?') => {
                            show_help = false;
                            needs_redraw = true;
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('?') => {
                        show_help = true;
                        needs_redraw = true;
                    }
                    KeyCode::Esc => break,
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
                        // don't treat shifted '?' as query input (already handled above)
                        if ch != '?'
                            && !key.modifiers.contains(KeyModifiers::CONTROL)
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

    disable_raw_mode().ok();
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();
    terminal.show_cursor().ok();

    Ok(TuiOutput {
        selected: result,
        last_query: query,
    })
}
