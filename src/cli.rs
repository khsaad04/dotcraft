use crate::error;

use std::env;
use std::path::PathBuf;
use std::process::exit;

#[derive(Debug)]
pub struct Cli {
    pub manifest_path: PathBuf,
    pub subcommand: SubCommand,
}

#[derive(Debug)]
pub enum SubCommand {
    Sync { force: bool, name: Option<String> },
    Link { force: bool, name: Option<String> },
    Generate { name: Option<String> },
}

const USAGE: &str = "
Usage: dotcraft [OPTION] <SUBCOMMAND>

Options:
    -m, --manifest <FILE>  Path to Manifest file [default: ./Manifest.toml]
    -h, --help             Print help

Subcommands:
    sync                   Symlink files and generate templates 
    link                   Symlink files
    generate               Generate templates";

const SYNC_USAGE: &str = "
Usage: dotcraft sync [OPTION] [NAME]

Options:
    -f, --force  Force remove existing files
    -h, --help   Print help";

const LINK_USAGE: &str = "
Usage: dotcraft link [OPTION] [NAME]

Options:
    -f, --force  Force remove existing files
    -h, --help   Print help";

const GENERATE_USAGE: &str = "
Usage: dotcraft generate [NAME]

Options:
    -h, --help  Print help";

impl Cli {
    pub fn try_parse() -> error::Result<Self> {
        let mut manifest_path = "./Manifest.toml".to_string();
        let mut subcommand: Option<SubCommand> = None;

        let mut args = env::args();
        let _program_name = args.next();

        while let Some(arg) = args.next() {
            if arg.starts_with('-') {
                match arg.as_str() {
                    "-h" | "--help" => {
                        println!("Dotfiles manager for unix-like operating systems\n{USAGE}");
                        exit(0);
                    }
                    "-m" | "--manifest" => {
                        if let Some(path) = args.next() {
                            manifest_path = path;
                        } else {
                            return Err(format!("missing required argument: FILE.\n{USAGE}").into());
                        }
                    }
                    _ => return Err(format!("invalid option {arg}.\n{USAGE}").into()),
                }
            } else {
                match arg.as_str() {
                    "sync" => {
                        let mut force = false;
                        let mut name: Option<String> = None;
                        for arg in args.by_ref() {
                            if arg.starts_with('-') {
                                match arg.as_str() {
                                    "-h" | "--help" => {
                                        println!(
                                            "Symlink files and generate templates\n{SYNC_USAGE}"
                                        );
                                        exit(0);
                                    }
                                    "-f" | "--force" => force = true,
                                    _ => {
                                        return Err(
                                            format!("invalid option {arg}.\n{SYNC_USAGE}").into()
                                        )
                                    }
                                }
                            } else {
                                name = Some(arg);
                            }
                        }
                        subcommand = Some(SubCommand::Sync { force, name });
                    }
                    "link" => {
                        let mut force = false;
                        let mut name: Option<String> = None;
                        for arg in args.by_ref() {
                            if arg.starts_with('-') {
                                match arg.as_str() {
                                    "-h" | "--help" => {
                                        println!("Symlink files\n{LINK_USAGE}");
                                        exit(0);
                                    }
                                    "-f" | "--force" => force = true,
                                    _ => {
                                        return Err(
                                            format!("invalid option {arg}.\n{LINK_USAGE}").into()
                                        )
                                    }
                                }
                            } else {
                                name = Some(arg);
                            }
                        }
                        subcommand = Some(SubCommand::Link { force, name });
                    }
                    "generate" => {
                        let mut name: Option<String> = None;
                        for arg in args.by_ref() {
                            if arg.starts_with('-') {
                                match arg.as_str() {
                                    "-h" | "--help" => {
                                        println!("Generate templates\n{GENERATE_USAGE}");
                                        exit(0);
                                    }
                                    _ => {
                                        return Err(format!(
                                            "invalid option {arg}.\n{GENERATE_USAGE}"
                                        )
                                        .into())
                                    }
                                }
                            } else {
                                name = Some(arg);
                            }
                        }
                        subcommand = Some(SubCommand::Generate { name });
                    }
                    _ => return Err(format!("invalid subcommand {arg}.\n{USAGE}").into()),
                }
            }
        }

        if let Some(subcommand) = subcommand {
            Ok(Cli {
                manifest_path: manifest_path.into(),
                subcommand,
            })
        } else {
            Err(format!("missing required argument: SUBCOMMAND\n{USAGE}").into())
        }
    }
}
