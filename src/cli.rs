use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about, long_about = None, arg_required_else_help=true)]
pub struct Cli {
    /// Sets a custom Manifest file
    #[arg(short, long, value_name = "FILE", default_value = "Manifest.toml")]
    pub manifest: PathBuf,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate and symlink all files
    Sync {
        /// Force remove existing files
        #[arg(short, long)]
        force: bool,
    },
    /// Symlink all files
    Link {
        /// Force remove existing files
        #[arg(short, long)]
        force: bool,
    },
    /// Generate all templates
    Generate,
}
