mod cli;
mod colors;

use clap::Parser;
use glob::glob;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    wallpaper: Option<String>,
    dark: Option<bool>,
    files: HashMap<String, File>,
}

#[derive(Debug, Deserialize, Serialize)]
struct File {
    target: String,
    dest: String,
    template: Option<String>,
}

type VarMap = HashMap<String, String>;

type Result<T> = std::result::Result<T, ()>;

fn entrypoint() -> Result<()> {
    let cli = cli::Cli::parse();

    // Parse Manifest file
    let manifest_path = cli.manifest.canonicalize().map_err(|err| {
        eprintln!(
            "ERROR: could not find {manifest_path}: {err}",
            manifest_path = &cli.manifest.display()
        );
    })?;
    let manifest_parent_dir = manifest_path.parent().unwrap();
    std::env::set_current_dir(manifest_parent_dir).map_err(|err| {
        eprintln!(
            "ERROR: could not change directory to {}: {err}",
            manifest_parent_dir.display()
        );
    })?;
    let manifest: Manifest =
        toml::from_str(&fs::read_to_string(&manifest_path).map_err(|err| {
            eprintln!(
                "ERROR: could not read file {}: {err}",
                manifest_path.display()
            );
        })?)
        .map_err(|err| {
            eprintln!(
                "ERROR: could not parse toml {}: {err}",
                manifest_path.display()
            );
        })?;

    // Generate color scheme from wallpaper
    let mut config: VarMap = HashMap::new();
    if let Some(wallpaper) = manifest.wallpaper {
        let wp_path = PathBuf::from(&wallpaper).canonicalize().map_err(|err| {
            eprintln!("ERROR: could not find {wallpaper}: {err}",);
        })?;
        config.insert("wallpaper".to_string(), wp_path.display().to_string());
        let scheme = manifest.dark.unwrap_or(true);
        colors::generate_material_colors(wp_path, scheme, &mut config)?;
    } else if has_templates(&manifest) {
        eprintln!("ERROR: `wallpaper` is not set. Needed for color scheme generation");
    } else {
        println!("WARNING: Skipping color scheme generation.");
    }

    // Execute commands
    match &cli.command {
        Some(cli::Commands::Sync {
            force,
            name: specified_name,
        }) => {
            if let Some(specified_name) = specified_name {
                if let Some(file) = manifest.files.get(specified_name) {
                    run_sync_command(file, &config, force)?;
                } else {
                    eprintln!("ERROR: `{}` not found", specified_name);
                    return Ok(());
                }
            } else {
                for (_, file) in manifest.files.iter() {
                    run_sync_command(file, &config, force)?;
                }
            }
        }
        Some(cli::Commands::Link {
            force,
            name: specified_name,
        }) => {
            if let Some(specified_name) = specified_name {
                if let Some(file) = manifest.files.get(specified_name) {
                    run_link_command(file, force)?;
                } else {
                    eprintln!("ERROR: `{}` not found", specified_name);
                    return Ok(());
                }
            } else {
                for (_, file) in manifest.files.iter() {
                    run_link_command(file, force)?;
                }
            }
        }
        Some(cli::Commands::Generate {
            name: specified_name,
        }) => {
            if let Some(specified_name) = specified_name {
                if let Some(file) = manifest.files.get(specified_name) {
                    run_generate_command(file, &config)?;
                } else {
                    eprintln!("ERROR: `{}` not found", specified_name);
                    return Ok(());
                }
            } else {
                for (_, file) in manifest.files.iter() {
                    run_generate_command(file, &config)?;
                }
            }
        }
        None => {
            unreachable!()
        }
    }
    Ok(())
}

fn resolve_home_dir(path: &str) -> String {
    let mut result = String::new();
    let home_dir = std::env::var("HOME").unwrap();
    result.push_str(&path.replace('~', &home_dir).replace("$HOME", &home_dir));
    result
}

fn run_sync_command(file: &File, config: &VarMap, force: &bool) -> Result<()> {
    let globbed_path = glob(&resolve_home_dir(&file.target)).map_err(|err| {
        eprintln!(
            "ERROR: could not parse target {target_path}: {err}",
            target_path = &file.target
        )
    })?;
    for entry in globbed_path {
        let entry = entry.unwrap();
        let dest_path = PathBuf::from(resolve_home_dir(&file.dest));
        let dest_path = if dest_path.is_dir() {
            dest_path.join(entry.iter().last().unwrap())
        } else {
            dest_path
        };
        if let Some(template_path) = &file.template {
            let template_path = PathBuf::from(template_path);
            if template_path.exists() {
                generate_template(&template_path, &entry, config)?;
            } else {
                eprintln!("ERROR: Template `{}` not found", &template_path.display())
            }
        }
        symlink_dir_all(&entry, &dest_path, force)?;
    }
    Ok(())
}

fn run_link_command(file: &File, force: &bool) -> Result<()> {
    let globbed_path = glob(&resolve_home_dir(&file.target)).map_err(|err| {
        eprintln!(
            "ERROR: could not parse target {target_path}: {err}",
            target_path = &file.target
        )
    })?;
    for entry in globbed_path {
        let entry = entry.unwrap();
        let dest_path = PathBuf::from(resolve_home_dir(&file.dest));
        let dest_path = if dest_path.is_dir() {
            dest_path.join(entry.iter().last().unwrap())
        } else {
            dest_path
        };
        symlink_dir_all(&entry, &dest_path, force)?;
    }
    Ok(())
}

fn run_generate_command(file: &File, config: &VarMap) -> Result<()> {
    let globbed_path = glob(&resolve_home_dir(&file.target)).map_err(|err| {
        eprintln!(
            "ERROR: could not parse target {target_path}: {err}",
            target_path = &file.target
        )
    })?;
    for entry in globbed_path {
        let entry = entry.unwrap();
        if let Some(template_path) = &file.template {
            let template_path = PathBuf::from(template_path);
            if template_path.exists() {
                generate_template(&template_path, &entry, config)?;
            } else {
                eprintln!("ERROR: Template `{}` not found", &template_path.display())
            }
        }
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

fn generate_template(
    template: &Path,
    target: &Path,
    config: &HashMap<String, String>,
) -> Result<()> {
    let data = fs::read_to_string(template).map_err(|err| {
        eprintln!("ERROR: could not read file {}: {err}", template.display());
    })?;

    let mut engine = upon::Engine::new();
    engine
        .add_template(template.to_str().unwrap(), &data)
        .map_err(|err| {
            eprintln!(
                "ERROR: could not add template {}: {err}",
                template.display()
            );
        })?;
    let rendered = engine
        .template(template.to_str().unwrap())
        .render(config)
        .to_string()
        .map_err(|err| {
            eprintln!(
                "ERROR: could not render template {}: {err}",
                template.display()
            );
        })?;

    fs::write(target, rendered).map_err(|err| {
        eprintln!(
            "ERROR: could not write to file {target_path}: {err}",
            target_path = target.display()
        );
    })?;
    println!("INFO: Generated template `{}`", template.display());
    Ok(())
}

fn symlink_dir_all(target: &Path, dest: &Path, force: &bool) -> Result<()> {
    if target.is_dir() {
        for entry in fs::read_dir(target).unwrap() {
            let entry = entry.unwrap();
            let dest = &dest.join(entry.path().file_name().unwrap());
            let dest_parent_dir = dest.parent().unwrap();
            if !dest_parent_dir.exists() {
                fs::create_dir_all(dest_parent_dir).map_err(|err| {
                    eprintln!(
                        "ERROR: could not create dir {}: {err}",
                        dest_parent_dir.display()
                    );
                })?;
            }
            symlink_dir_all(&entry.path(), dest, force)?;
        }
    } else {
        symlink_file(target, dest, force)?;
    }
    Ok(())
}

fn symlink_file(target: &Path, dest: &Path, force: &bool) -> Result<()> {
    match symlink(target, dest) {
        Ok(()) => {
            println!(
                "INFO: Symlinked `{}` -> `{}`",
                target.display(),
                dest.display()
            );
        }
        Err(err) => {
            match err.kind() {
                io::ErrorKind::AlreadyExists => {
                    if *force {
                        println!(
                            "WARNING: Destination `{}` already exists. Removing",
                            dest.display()
                        );
                        std::fs::remove_file(dest).map_err(|err| {
                            eprintln!("ERROR: could not remove file {}: {err}", dest.display());
                        })?;
                        symlink_file(target, dest, force)?;
                        println!(
                            "INFO: Symlinked `{}` -> `{}`",
                            target.display(),
                            dest.display()
                        );
                        return Ok(());
                    }
                    if dest.is_symlink() {
                        let symlink_origin = dest.canonicalize().unwrap();
                        if target.canonicalize().unwrap() == symlink_origin {
                            println!("INFO: Skipped symlinking `{}`. Up to date.", dest.display());
                        } else {
                            println!(
                                "WARNING: Destination `{}` is symlinked to `{}`. Resolve manually.",
                                dest.display(),
                                symlink_origin.display()
                            );
                        }
                    } else {
                        println!("WARNING: Destination `{}` exists but it's not a symlink. Resolve manually", dest.display());
                    }
                }
                _ => {
                    eprintln!(
                        "ERROR: Failed to symlink `{}` -> `{}`. {}",
                        target.display(),
                        dest.display(),
                        err
                    );
                }
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match entrypoint() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
