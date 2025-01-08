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
    /// Generate templates and symlink files
    Sync {
        /// Force remove existing files
        #[arg(short, long)]
        force: bool,
        name: Option<String>,
    },
    /// Symlink files
    Link {
        /// Force remove existing files
        #[arg(short, long)]
        force: bool,
        name: Option<String>,
    },
    /// Generate templates
    Generate {
        #[arg(short, long)]
        name: Option<String>,
    },
}
