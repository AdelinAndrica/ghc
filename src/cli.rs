// src/cli.rs
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum SortBy {
    Updated,
    Name,
    Stars,
}

#[derive(Parser, Debug)]
#[command(
    name = "ghc",
    about = "Interactive GitHub repo picker + clone",
    version
)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Option<Cmd>,

    /// Clone using SSH
    #[arg(long, conflicts_with = "https")]
    pub ssh: bool,

    /// Clone using HTTPS
    #[arg(long, conflicts_with = "ssh")]
    pub https: bool,

    /// Only show repos owned by the authenticated user
    #[arg(long)]
    pub owned: bool,

    /// Include archived repositories (archived repos are hidden by default)
    #[arg(long)]
    pub archived: bool,

    /// Sorting for repositories: updated | name | stars
    #[arg(long, value_enum, default_value_t = SortBy::Updated)]
    pub sort: SortBy,
}

#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// Authenticate with GitHub using device flow and store token locally
    Login,
    /// Print the currently stored token source (debug)
    AuthStatus,
    /// Remove locally stored token
    Logout,

    /// Non-interactive clone mode (script-friendly)
    ///
    /// Example:
    ///   ghc clone owner/repo
    Clone {
        /// GitHub repo in the form owner/repo
        repo: String,
    },
}

#[derive(Copy, Clone, Debug)]
pub enum CloneProtocol {
    Https,
    Ssh,
}

impl CloneProtocol {
    pub fn from_flags(ssh: bool, https: bool) -> Self {
        if ssh {
            Self::Ssh
        } else if https {
            Self::Https
        } else {
            // default
            Self::Https
        }
    }
}
