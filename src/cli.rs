use crate::error;

use std::env;
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
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
Usage: dotman [OPTION] <SUBCOMMAND>

Options:
    -m, --manifest <FILE>  Path to Manifest file [default: ./Manifest.toml]
    -h, --help             Print help

Subcommands:
    sync                   Symlink files and generate templates 
    link                   Symlink files
    generate               Generate templates";

const SYNC_USAGE: &str = "
Usage: dotman sync [OPTION] [NAME]

Options:
    -f, --force  Force remove existing files
    -h, --help   Print help";

const LINK_USAGE: &str = "
Usage: dotman link [OPTION] [NAME]

Options:
    -f, --force  Force remove existing files
    -h, --help   Print help";

const GENERATE_USAGE: &str = "
Usage: dotman generate [NAME]

Options:
    -h, --help  Print help";

impl Cli {
    pub fn try_parse() -> error::Result<Self> {
        let mut manifest_path = OsString::from("Manifest.toml");
        let mut subcommand: Option<SubCommand> = None;

        let mut args = env::args_os();
        let _program_name = args.next();

        while let Some(arg) = args.next() {
            let arg = arg.as_bytes();
            if arg.starts_with(b"-") {
                match arg {
                    b"-h" | b"--help" => {
                        println!("Dotfiles manager for unix-like operating systems\n{USAGE}");
                        exit(0);
                    }
                    b"-m" | b"--manifest" => {
                        if let Some(path) = args.next() {
                            manifest_path = path;
                        } else {
                            return Err(format!("missing required argument: PATH.\n{USAGE}").into());
                        }
                    }
                    _ => {
                        return Err(format!(
                            "invalid flag {}.\n{USAGE}",
                            String::from_utf8_lossy(arg)
                        )
                        .into())
                    }
                }
            } else {
                match arg {
                    b"sync" => {
                        let mut force = false;
                        let mut name: Option<String> = None;
                        for arg in args.by_ref() {
                            let arg = arg.as_bytes();
                            if arg.starts_with(b"-") {
                                match arg {
                                    b"-h" | b"--help" => {
                                        println!(
                                            "Symlink files and generate templates\n{SYNC_USAGE}"
                                        );
                                        exit(0);
                                    }
                                    b"-f" | b"--force" => force = true,
                                    _ => {
                                        return Err(format!(
                                            "invalid flag {}.\n{SYNC_USAGE}",
                                            String::from_utf8_lossy(arg)
                                        )
                                        .into())
                                    }
                                }
                            } else {
                                name = Some(String::from_utf8_lossy(arg).to_string());
                            }
                        }
                        subcommand = Some(SubCommand::Sync { force, name });
                    }
                    b"link" => {
                        let mut force = false;
                        let mut name: Option<String> = None;
                        for arg in args.by_ref() {
                            let arg = arg.as_bytes();
                            if arg.starts_with(b"-") {
                                match arg {
                                    b"-h" | b"--help" => {
                                        println!("Symlink files\n{LINK_USAGE}");
                                        exit(0);
                                    }
                                    b"-f" | b"--force" => force = true,
                                    _ => {
                                        return Err(format!(
                                            "invalid flag {}.\n{LINK_USAGE}",
                                            String::from_utf8_lossy(arg)
                                        )
                                        .into())
                                    }
                                }
                            } else {
                                name = Some(String::from_utf8_lossy(arg).to_string());
                            }
                        }
                        subcommand = Some(SubCommand::Link { force, name });
                    }
                    b"generate" => {
                        let mut name: Option<String> = None;
                        for arg in args.by_ref() {
                            let arg = arg.as_bytes();
                            if arg.starts_with(b"-") {
                                match arg {
                                    b"-h" | b"--help" => {
                                        println!("Generate templates\n{GENERATE_USAGE}");
                                        exit(0);
                                    }
                                    _ => {
                                        return Err(format!(
                                            "invalid flag {}.\n{GENERATE_USAGE}",
                                            String::from_utf8_lossy(arg)
                                        )
                                        .into())
                                    }
                                }
                            } else {
                                name = Some(String::from_utf8_lossy(arg).to_string());
                            }
                        }
                        subcommand = Some(SubCommand::Generate { name });
                    }
                    _ => {
                        return Err(format!(
                            "invalid subcommand {}.\n{USAGE}",
                            String::from_utf8_lossy(arg)
                        )
                        .into())
                    }
                }
            }
        }

        if let Some(subcommand) = subcommand {
            Ok(Cli {
                manifest_path: manifest_path.into(),
                subcommand,
            })
        } else {
            Err(format!("missing arguments.\n{USAGE}").into())
        }
    }
}
