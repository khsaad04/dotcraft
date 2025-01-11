mod colors;
mod error;

use colors::generate_material_colors;
use error::Result;

use glob::glob;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env::Args,
    fs, io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::exit,
};

#[derive(Debug, Deserialize)]
struct Manifest {
    wallpaper: Option<String>,
    theme: Option<bool>,
    files: HashMap<String, File>,
}

#[derive(Debug, Deserialize)]
struct File {
    target: String,
    dest: String,
    template: Option<String>,
}

type VarMap = HashMap<String, String>;

const MAIN_USAGE: &str = "Usage: dotman [OPTION] <SUBCOMMAND>

Options:
  -m, --manifest <PATH>  custom path to manifest file [default: Manifest.toml]
  -h, --help             show this help message

Subcommands:
  sync      symlink files and generate templates 
  link      symlink files
  generate  generate templates";

const SYNC_USAGE: &str = "Symlink files and generate templates

Usage: dotman sync [OPTIONS] [NAME]

Arguments:
  [NAME]       name of the specific file(s) to sync

Options:
  -f, --force  force remove existing files
  -h, --help   show this help message";

const LINK_USAGE: &str = "Symlink files

Usage: dotman sync [OPTIONS] [NAME]

Arguments:
  [NAME]       name of the specific file(s) to link

Options:
  -f, --force  force remove existing files
  -h, --help   show this help message";

const GENERATE_USAGE: &str = "Generate templates

Usage: dotman sync [OPTIONS] [NAME]

Arguments:
  [NAME]       name of the specific file(s) to generate

Options:
  -h, --help   show this help message";

impl TryFrom<&Path> for Manifest {
    type Error = error::Error;
    fn try_from(value: &Path) -> std::result::Result<Self, Self::Error> {
        let manifest_path = value
            .canonicalize()
            .map_err(|err| format!("could not find {path}: {err}", path = &value.display()))?;
        let manifest_parent_dir = manifest_path.parent().unwrap();
        std::env::set_current_dir(manifest_parent_dir).map_err(|err| {
            format!(
                "could not change directory to {path}: {err}",
                path = &manifest_parent_dir.display()
            )
        })?;
        let manifest: Manifest =
            toml::from_str(&fs::read_to_string(&manifest_path).map_err(|err| {
                format!(
                    "could not read file {path}: {err}",
                    path = &manifest_path.display()
                )
            })?)
            .map_err(|err| {
                format!(
                    "could not parse toml {path}: {err}",
                    path = &manifest_path.display()
                )
            })?;
        Ok(manifest)
    }
}

fn main() {
    let mut args = std::env::args();
    let _program_name = args.next();

    let mut config: VarMap = HashMap::new();
    if let Err(err) = parse_arguments(&mut args, &mut config) {
        eprintln!("{err}");
        exit(1);
    }
}

fn parse_arguments(args: &mut Args, config: &mut VarMap) -> Result<()> {
    let mut manifest_path = String::new();
    if let Some(arg) = args.next() {
        if arg.starts_with('-') {
            match arg.as_str() {
                "-m" | "--manifest" => {
                    manifest_path.push_str(&args.next().unwrap());
                }
                "-h" | "--help" => {
                    println!("{MAIN_USAGE}");
                    return Ok(());
                }
                _ => {
                    return Err(format!("flag {arg} not found.\n{MAIN_USAGE}").into());
                }
            }
        } else {
            if manifest_path.is_empty() {
                manifest_path.push_str("Manifest.toml");
            }
            let manifest = Manifest::try_from(Path::new(&manifest_path))?;
            create_color_palette(&manifest.wallpaper, config, &manifest)?;
            match arg.as_str() {
                "sync" => {
                    let mut force = false;
                    let mut name: Option<String> = None;
                    if let Some(arg) = args.next() {
                        if arg.starts_with('-') {
                            match arg.as_str() {
                                "-f" | "--force" => {
                                    force = true;
                                    name = args.next();
                                }
                                "-h" | "--help" => {
                                    println!("{SYNC_USAGE}");
                                    return Ok(());
                                }
                                _ => {
                                    return Err(
                                        format!("flag {arg} not found.\n{SYNC_USAGE}").into()
                                    );
                                }
                            }
                        } else {
                            name = Some(arg);
                        }
                    }
                    if let Some(name) = name {
                        if let Some(file) = manifest.files.get(&name) {
                            symlink_files(file, force)?;
                            if file.template.is_some() {
                                generate_template(file, config)?;
                            }
                        } else {
                            return Err(format!("could not find {}", &name).into());
                        }
                    } else {
                        for (_, file) in manifest.files.iter() {
                            symlink_files(file, force)?;
                            if file.template.is_some() {
                                generate_template(file, config)?;
                            }
                        }
                    }
                }
                "link" => {
                    let mut force = false;
                    let mut name: Option<String> = None;
                    if let Some(arg) = args.next() {
                        if arg.starts_with('-') {
                            match arg.as_str() {
                                "-f" | "--force" => {
                                    force = true;
                                    name = args.next();
                                }
                                "-h" | "--help" => {
                                    println!("{LINK_USAGE}");
                                    return Ok(());
                                }
                                _ => {
                                    return Err(
                                        format!("flag {arg} not found.\n{LINK_USAGE}").into()
                                    );
                                }
                            }
                        } else {
                            name = Some(arg);
                        }
                    }
                    if let Some(name) = name {
                        if let Some(file) = manifest.files.get(&name) {
                            symlink_files(file, force)?;
                        } else {
                            return Err(format!("could not find {}", &name).into());
                        }
                    } else {
                        for (_, file) in manifest.files.iter() {
                            symlink_files(file, force)?;
                        }
                    }
                }
                "generate" => {
                    let mut name: Option<String> = None;
                    if let Some(arg) = args.next() {
                        if arg.starts_with('-') {
                            match arg.as_str() {
                                "-h" | "--help" => {
                                    println!("{GENERATE_USAGE}");
                                    return Ok(());
                                }
                                _ => {
                                    return Err(
                                        format!("flag {arg} not found.\n{GENERATE_USAGE}").into()
                                    );
                                }
                            }
                        } else {
                            name = Some(arg);
                        }
                    }
                    if let Some(name) = name {
                        if let Some(file) = manifest.files.get(&name) {
                            if file.template.is_some() {
                                generate_template(file, config)?;
                            }
                        } else {
                            return Err(format!("could not find {}", &name).into());
                        }
                    } else {
                        for (_, file) in manifest.files.iter() {
                            if file.template.is_some() {
                                generate_template(file, config)?;
                            }
                        }
                    }
                }
                _ => {
                    return Err(format!("subcommand {arg} not found.\n{MAIN_USAGE}").into());
                }
            }
        }
    }
    Ok(())
}

fn create_color_palette(
    path: &Option<String>,
    config: &mut VarMap,
    manifest: &Manifest,
) -> Result<()> {
    if let Some(wallpaper) = path {
        let wp_path = PathBuf::from(&wallpaper)
            .canonicalize()
            .map_err(|err| format!("could not find {wallpaper}: {err}"))?;
        config.insert("wallpaper".to_string(), wp_path.display().to_string());
        let theme = manifest.theme.unwrap_or(true);
        generate_material_colors(&wp_path, theme, config)?;
    } else if has_templates(manifest) {
        return Err("could not generate color palette: `wallpaper` is not set.".into());
    } else {
        println!("WARNING: Skipping color scheme generation.");
    }
    Ok(())
}

fn has_templates(manifest: &Manifest) -> bool {
    for (_, file) in manifest.files.iter() {
        if file.template.is_some() {
            return true;
        }
    }
    false
}

fn symlink_files(file: &File, force: bool) -> Result<()> {
    let globbed_path = glob(&resolve_home_dir(&file.target)?)
        .map_err(|err| format!("could not parse target {path}: {err}", path = &file.target))?;
    for entry in globbed_path {
        let entry = entry?.canonicalize()?;
        let dest_path = PathBuf::from(resolve_home_dir(&file.dest)?);
        let dest_path = if dest_path.is_dir() {
            dest_path.join(entry.iter().last().unwrap())
        } else {
            dest_path
        };
        symlink_dir_all(&entry, &dest_path, force)?;
    }
    Ok(())
}

fn resolve_home_dir(path: &str) -> Result<String> {
    let mut result = String::new();
    let home_dir = std::env::var("HOME")?;
    result.push_str(&path.replace('~', &home_dir).replace("$HOME", &home_dir));
    Ok(result)
}

fn symlink_dir_all(target: &Path, dest: &Path, force: bool) -> Result<()> {
    if target.is_dir() {
        for entry in fs::read_dir(target)? {
            let entry = entry?;
            let dest = &dest.join(entry.path().file_name().unwrap());
            let dest_parent_dir = dest.parent().unwrap();
            if !dest_parent_dir.exists() {
                fs::create_dir_all(dest_parent_dir).map_err(|err| {
                    format!(
                        "could not create dir {path}: {err}",
                        path = &dest_parent_dir.display()
                    )
                })?;
            }
            symlink_dir_all(&entry.path(), dest, force)?;
        }
    } else {
        symlink_file(target, dest, force)?;
    }
    Ok(())
}

fn symlink_file(target: &Path, dest: &Path, force: bool) -> Result<()> {
    match symlink(target, dest) {
        Ok(()) => {
            println!("INFO: Symlinked {} to {}", target.display(), dest.display());
        }
        Err(err) => {
            match err.kind() {
                io::ErrorKind::AlreadyExists => {
                    if force {
                        println!(
                            "WARNING: Destination {} already exists. Removing",
                            dest.display()
                        );
                        std::fs::remove_file(dest).map_err(|err| {
                            format!(
                                "could not remove file {path}: {err}",
                                path = &dest.display()
                            )
                        })?;
                        symlink(target, dest)?;
                        println!("INFO: Symlinked {} to {}", target.display(), dest.display());
                    } else if dest.is_symlink() {
                        let symlink_origin = dest.canonicalize()?;
                        if target.canonicalize()? == symlink_origin {
                            println!("INFO: Skipped symlinking {}. Up to date.", dest.display());
                        } else {
                            println!(
                                "WARNING: Destination {} is symlinked to {}. Resolve manually.",
                                dest.display(),
                                symlink_origin.display()
                            );
                        }
                    } else {
                        println!("WARNING: Destination {} exists but it's not a symlink. Resolve manually", dest.display());
                    }
                }
                _ => {
                    return Err(format!(
                        "could not symlink {target_path} to {dest_path}: {err}",
                        target_path = &target.display(),
                        dest_path = &dest.display(),
                    )
                    .into());
                }
            }
        }
    }
    Ok(())
}

fn generate_template(file: &File, config: &VarMap) -> Result<()> {
    let target_path = PathBuf::from(&file.target).canonicalize().map_err(|err| {
        format!(
            "cannot generate template into {path}: {err}",
            path = &file.target
        )
    })?;
    if let Some(template_path) = &file.template {
        let template_path = PathBuf::from(template_path);
        if template_path.exists() {
            let data = fs::read_to_string(&template_path).map_err(|err| {
                format!(
                    "could not read file {path}: {err}",
                    path = &template_path.display()
                )
            })?;

            let mut engine = upon::Engine::new();
            engine
                .add_template(template_path.to_str().unwrap(), &data)
                .map_err(|err| {
                    format!(
                        "could not add template {path}: {err}",
                        path = &template_path.display()
                    )
                })?;
            let rendered = engine
                .template(template_path.to_str().unwrap())
                .render(config)
                .to_string()
                .map_err(|err| {
                    format!(
                        "could not render template {path}: {err}",
                        path = &template_path.display()
                    )
                })?;

            fs::write(&target_path, rendered).map_err(|err| {
                format!(
                    "could not write to file {path}: {err}",
                    path = &target_path.display()
                )
            })?;
            println!("INFO: Generated template {}", template_path.display());
        } else {
            return Err(format!(
                "could not find template {path}",
                path = &template_path.display()
            )
            .into());
        }
    }
    Ok(())
}
