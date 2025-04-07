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

const USAGE: &str = "Usage: dotman [OPTION] <SUBCOMMAND>

Options:
  -m, --manifest <PATH>  custom path to manifest file [default: Manifest.toml]
  -h, --help             show this help message

Subcommands:
  sync      [-f | --force] [NAME] symlink files and generate templates 
  link      [-f | --force] [NAME] symlink files
  generate  [NAME] generate templates";

impl Cli {
    pub fn try_parse() -> error::Result<Self> {
        let mut manifest_path = "Manifest.toml".to_string();
        let mut subcommand: Option<SubCommand> = None;

        let mut args = env::args_os();
        let _program_name = args.next();

        while let Some(arg) = args.next() {
            let arg = arg.to_str().unwrap();
            if arg.contains('-') {
                match arg {
                    "-h" | "--help" => {
                        println!("{USAGE}");
                        exit(0);
                    }
                    "-m" | "--manifest" => {
                        if let Some(path) = args.next() {
                            manifest_path = path
                                .into_string()
                                .map_err(|_| "failed to convert OsString to  String")?;
                        } else {
                            return Err(format!("missing required argument path.\n{USAGE}").into());
                        }
                    }
                    _ => return Err(format!("invalid flag {arg}.\n{USAGE}").into()),
                }
            } else {
                match arg {
                    "sync" => {
                        let mut force = false;
                        let mut name: Option<String> = None;
                        for arg in args.by_ref() {
                            let arg = arg.to_str().unwrap();
                            if arg.starts_with('-') {
                                match arg {
                                    "-h" | "--help" => {
                                        println!("{USAGE}");
                                        exit(0);
                                    }
                                    "-f" | "--force" => force = true,
                                    _ => return Err(format!("invalid flag {arg}.\n{USAGE}").into()),
                                }
                            } else {
                                name = Some(arg.to_string());
                            }
                        }
                        subcommand = Some(SubCommand::Sync { force, name });
                    }
                    "link" => {
                        let mut force = false;
                        let mut name: Option<String> = None;
                        for arg in args.by_ref() {
                            let arg = arg.to_str().unwrap();
                            if arg.starts_with('-') {
                                match arg {
                                    "-h" | "--help" => {
                                        println!("{USAGE}");
                                        exit(0);
                                    }
                                    "-f" | "--force" => force = true,
                                    _ => return Err(format!("invalid flag {arg}.\n{USAGE}").into()),
                                }
                            } else {
                                name = Some(arg.to_string());
                            }
                        }
                        subcommand = Some(SubCommand::Link { force, name });
                    }
                    "generate" => {
                        let mut name: Option<String> = None;
                        for arg in args.by_ref() {
                            let arg = arg.to_str().unwrap();
                            if arg.starts_with('-') {
                                match arg {
                                    "-h" | "--help" => {
                                        println!("{USAGE}");
                                        exit(0);
                                    }
                                    _ => return Err(format!("invalid flag {arg}.\n{USAGE}").into()),
                                }
                            } else {
                                name = Some(arg.to_string());
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
            Err(format!("missing arguments.\n{USAGE}").into())
        }
    }
}
