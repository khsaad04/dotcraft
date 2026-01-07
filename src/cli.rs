use crate::Result;

use flagge::Lexer;
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
Usage: tread [OPTION] <SUBCOMMAND>

Options:
    -m, --manifest <FILE>  Path to Manifest file [default: ./Manifest.toml]
    -h, --help             Print help

Subcommands:
    sync      Symlink files and generate templates 
    link      Symlink files
    generate  Generate templates";

const SYNC_USAGE: &str = "
Usage: tread sync [OPTION] [NAME]

Options:
    -f, --force  Force remove existing files
    -h, --help   Print help";

const LINK_USAGE: &str = "
Usage: tread link [OPTION] [NAME]

Options:
    -f, --force  Force remove existing files
    -h, --help   Print help";

const GENERATE_USAGE: &str = "
Usage: tread generate [NAME]

Options:
    -h, --help  Print help";

impl Cli {
    pub fn try_parse() -> Result<Self> {
        let mut manifest_path = PathBuf::from("Manifest.toml");
        let mut subcommand: Option<SubCommand> = None;

        let mut lexer = Lexer::from_env();
        while let Some(arg) = lexer.next_token()? {
            use flagge::Token::*;
            match arg {
                ShortFlag('h') | LongFlag("help") => {
                    println!("Simple template generator and dotfiles manager\n{USAGE}");
                    exit(0);
                }
                ShortFlag('m') | LongFlag("manifest") => {
                    if let Some(path) = lexer.get_value() {
                        manifest_path = path.into();
                    } else {
                        return Err(format!("missing required argument: FILE\n{USAGE}").into());
                    }
                }
                Value(ref val) => match val.as_os_str().as_bytes() {
                    b"sync" => {
                        let mut force = false;
                        let mut name: Option<String> = None;
                        while let Some(arg) = lexer.next_token()? {
                            match arg {
                                ShortFlag('h') | LongFlag("help") => {
                                    println!("Symlink files and generate templates\n{SYNC_USAGE}");
                                    exit(0);
                                }
                                ShortFlag('f') | LongFlag("force") => force = true,
                                Value(val) => {
                                    name = Some(val.into_string().map_err(|err| {
                                        format!(
                                            "Unexpected argument in {}",
                                            String::from_utf8_lossy(err.as_os_str().as_bytes())
                                        )
                                    })?)
                                }
                                _ => {
                                    return Err(format!("invalid option {arg}\n{SYNC_USAGE}").into())
                                }
                            }
                        }
                        subcommand = Some(SubCommand::Sync { force, name });
                    }
                    b"link" => {
                        let mut force = false;
                        let mut name: Option<String> = None;
                        while let Some(arg) = lexer.next_token()? {
                            match arg {
                                ShortFlag('h') | LongFlag("help") => {
                                    println!("Symlink files\n{LINK_USAGE}");
                                    exit(0);
                                }
                                ShortFlag('f') | LongFlag("force") => force = true,
                                Value(val) => {
                                    name = Some(val.into_string().map_err(|err| {
                                        format!(
                                            "Unexpected argument in {}",
                                            String::from_utf8_lossy(err.as_os_str().as_bytes())
                                        )
                                    })?)
                                }
                                _ => {
                                    return Err(format!("invalid option {arg}\n{LINK_USAGE}").into())
                                }
                            }
                        }
                        subcommand = Some(SubCommand::Link { force, name });
                    }
                    b"generate" => {
                        let mut name: Option<String> = None;
                        while let Some(arg) = lexer.next_token()? {
                            match arg {
                                ShortFlag('h') | LongFlag("help") => {
                                    println!("Generate templates\n{GENERATE_USAGE}");
                                    exit(0);
                                }
                                Value(val) => {
                                    name = Some(val.into_string().map_err(|err| {
                                        format!(
                                            "Unexpected argument in {}",
                                            String::from_utf8_lossy(err.as_os_str().as_bytes())
                                        )
                                    })?)
                                }
                                _ => {
                                    return Err(
                                        format!("invalid option {arg}\n{GENERATE_USAGE}").into()
                                    )
                                }
                            }
                        }
                        subcommand = Some(SubCommand::Generate { name });
                    }
                    _ => return Err(format!("invalid subcommand {arg}\n{USAGE}").into()),
                },
                _ => return Err(format!("invalid argument {arg}\n{USAGE}").into()),
            }
        }

        if let Some(subcommand) = subcommand {
            Ok(Cli {
                manifest_path,
                subcommand,
            })
        } else {
            Err(format!("missing required argument: SUBCOMMAND\n{USAGE}").into())
        }
    }
}
