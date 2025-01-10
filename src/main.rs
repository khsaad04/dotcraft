mod cli;
mod colors;
mod error;

use cli::Cli;
use colors::generate_material_colors;
use error::Result;

use clap::Parser;
use glob::glob;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs, io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::exit,
};

#[derive(Debug, Deserialize)]
struct Manifest {
    wallpaper: Option<String>,
    dark: Option<bool>,
    files: HashMap<String, File>,
}

#[derive(Debug, Deserialize)]
struct File {
    target: String,
    dest: String,
    template: Option<String>,
}

type VarMap = HashMap<String, String>;

fn main() {
    let cli = cli::Cli::parse();
    let mut config: VarMap = HashMap::new();
    match parse_manifest_file(&cli.manifest) {
        Ok(manifest) => {
            if let Err(err) = parse_wallpaper(&manifest.wallpaper, &mut config, &manifest) {
                eprintln!("{err}");
                exit(1);
            };
            if let Err(err) = execute_subcommands(&cli, &manifest, &config) {
                eprintln!("{err}");
                exit(1);
            };
        }
        Err(err) => {
            eprintln!("{err}");
            exit(1);
        }
    }
}

fn parse_manifest_file(path: &Path) -> Result<Manifest> {
    let manifest_path = path
        .canonicalize()
        .map_err(|err| format!("could not find {path}: {err}", path = &path.display()))?;
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

fn parse_wallpaper(path: &Option<String>, config: &mut VarMap, manifest: &Manifest) -> Result<()> {
    if let Some(wallpaper) = path {
        let wp_path = PathBuf::from(&wallpaper)
            .canonicalize()
            .map_err(|err| format!("could not find {wallpaper}: {err}"))?;
        config.insert("wallpaper".to_string(), wp_path.display().to_string());
        let scheme = manifest.dark.unwrap_or(true);
        generate_material_colors(&wp_path, &scheme, config)?;
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

fn execute_subcommands(cli: &Cli, manifest: &Manifest, config: &VarMap) -> Result<()> {
    if let Some(cli::Commands::Sync { force, name })
    | Some(cli::Commands::Link { force, name })
    | Some(cli::Commands::Generate { force, name }) = &cli.command
    {
        if let Some(name) = name {
            if let Some(file) = manifest.files.get(name.as_str()) {
                symlink_files(file, *force)?;
                if file.template.is_some() {
                    generate_template(file, config)?;
                }
            } else {
                return Err(format!("could not find {name}").into());
            }
        } else {
            for (_, file) in manifest.files.iter() {
                symlink_files(file, *force)?;
                if file.template.is_some() {
                    generate_template(file, config)?;
                }
            }
        }
    }
    Ok(())
}

fn symlink_files(file: &File, force: bool) -> Result<()> {
    let globbed_path = glob(&resolve_home_dir(&file.target)?)
        .map_err(|err| format!("could not parse target {path}: {err}", path = &file.target))?;
    for entry in globbed_path {
        let entry = entry?;
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
