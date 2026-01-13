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

    /// Clone using SSH instead of HTTPS
    #[arg(long)]
    pub ssh: bool,

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
}
