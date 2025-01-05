mod cli;
mod colors;

use clap::Parser;
use color_eyre::eyre::{self, Context};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    wallpaper: Option<PathBuf>,
    dark: Option<bool>,
    files: HashMap<String, File>,
}

type VarMap = HashMap<String, String>;

#[derive(Debug, Deserialize, Serialize)]
struct File {
    target: String,
    dest: Option<PathBuf>,
    template: Option<PathBuf>,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli = cli::Cli::parse();

    // Parse Manifest file
    let manifest_path = cli
        .manifest
        .canonicalize()
        .context(format!("`{}` not found", &cli.manifest.display()))?;
    std::env::set_current_dir(manifest_path.parent().unwrap())?;
    let manifest: Manifest = toml::from_str(
        &fs::read_to_string(manifest_path).context("Failed to read file Manifest.toml")?,
    )
    .context("Failed to parse Manifest.toml")?;

    // Generate color scheme from wallpaper
    let mut config: VarMap = HashMap::new();
    if let Some(wallpaper) = manifest.wallpaper {
        let wp_path = wallpaper
            .canonicalize()
            .context(format!("Wallpaper `{}` not found", wallpaper.display()))?;
        config.insert("wallpaper".to_string(), wp_path.display().to_string());
        let scheme = manifest.dark.unwrap_or(true);
        colors::generate_material_colors(wp_path, scheme, &mut config)?;
    } else if has_templates(&manifest) {
        panic!("`wallpaper` is not set. Needed for color scheme generation");
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
                    panic!("`{}` not found", specified_name);
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
                    panic!("`{}` not found", specified_name);
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
                    panic!("`{}` not found", specified_name);
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

fn parse_paths(entry: &Path, file: &File) -> eyre::Result<(PathBuf, PathBuf)> {
    let target_path = PathBuf::from(&entry)
        .canonicalize()
        .context(format!("Target `{}` not found", &entry.display()))?;
    let home_dir = PathBuf::from(std::env::var("HOME")?);
    let dest_path = if let Some(dest) = &file.dest {
        let dest = PathBuf::from(dest);
        if dest.is_dir() {
            home_dir.join(dest).join(entry.iter().last().unwrap())
        } else {
            home_dir.join(dest)
        }
    } else {
        home_dir.join(entry)
    };
    if !dest_path.parent().unwrap().exists() {
        fs::create_dir_all(dest_path.parent().unwrap())?;
    }
    Ok((target_path, dest_path))
}

fn run_sync_command(file: &File, config: &VarMap, force: &bool) -> eyre::Result<()> {
    let globbed_path = glob(&file.target).context(format!(
        "Failed to parse target `{}`. Invalid glob pattern",
        &file.target
    ))?;
    for entry in globbed_path {
        let entry = entry?;
        let (target_path, dest_path) = parse_paths(&entry, file)?;
        if let Some(template_path) = &file.template {
            generate_template(
                config,
                &PathBuf::from(template_path)
                    .canonicalize()
                    .context(format!("Template `{}` not found", &template_path.display()))?,
                &target_path,
            )?;
        }
        symlink_dir_all(&target_path, &dest_path, force)?;
    }
    Ok(())
}

fn run_link_command(file: &File, force: &bool) -> eyre::Result<()> {
    let globbed_path = glob(&file.target).context(format!(
        "Failed to parse target `{}`. Invalid glob pattern",
        &file.target
    ))?;
    for entry in globbed_path {
        let entry = entry?;
        let (target_path, dest_path) = parse_paths(&entry, file)?;
        symlink_dir_all(&target_path, &dest_path, force)?;
    }
    Ok(())
}

fn run_generate_command(file: &File, config: &VarMap) -> eyre::Result<()> {
    let globbed_path = glob(&file.target).context(format!(
        "Failed to parse target `{}`. Invalid glob pattern",
        &file.target
    ))?;
    for entry in globbed_path {
        let entry = entry?;
        let (target_path, _) = parse_paths(&entry, file)?;
        if let Some(template_path) = &file.template {
            generate_template(
                config,
                &PathBuf::from(template_path)
                    .canonicalize()
                    .context(format!("Template `{}` not found", &template_path.display()))?,
                &target_path,
            )?;
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
    config: &HashMap<String, String>,
    template: &Path,
    target: &Path,
) -> eyre::Result<()> {
    let data = fs::read_to_string(template)
        .context(format!("Failed to parse template `{}`", template.display()))?;

    let mut engine = upon::Engine::new();
    engine
        .add_template(template.to_str().unwrap(), &data)
        .context(format!(
            "Failed to add template `{}` to template engine",
            template.display()
        ))?;
    let rendered = engine
        .template(template.to_str().unwrap())
        .render(config)
        .to_string()
        .context(format!(
            "Failed to render template `{}`",
            template.display()
        ))?;

    fs::write(target, rendered)?;
    println!("INFO: Generated `{}` template", template.display());
    Ok(())
}

fn symlink_dir_all(target: &Path, dest: &Path, force_flag: &bool) -> eyre::Result<()> {
    if target.is_dir() {
        for entry in fs::read_dir(target)? {
            let entry = entry?;
            let dest = &dest.join(entry.path().file_name().unwrap());
            if !dest.parent().unwrap().exists() {
                fs::create_dir_all(dest.parent().unwrap())?;
            }
            symlink_dir_all(&entry.path(), dest, force_flag)?;
        }
    } else {
        symlink_file(target, dest, force_flag)?;
    }
    Ok(())
}

fn symlink_file(target: &Path, dest: &Path, force_flag: &bool) -> eyre::Result<()> {
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
                    if *force_flag {
                        std::fs::remove_file(dest)?;
                        println!(
                            "WARNING: Destination `{}` already exists. Removing",
                            dest.display()
                        );
                        symlink(target, dest)?;
                        println!(
                            "INFO: Symlinked `{}` -> `{}`",
                            target.display(),
                            dest.display()
                        );
                        return Ok(());
                    }
                    if dest.is_symlink() {
                        let original_path = dest.canonicalize()?;
                        if target == original_path {
                            println!(
                                "WARNING: Destination `{}` already symlinked. Skipping",
                                dest.display()
                            );
                        } else {
                            println!(
                                "ERROR: Destination `{}` is symlinked to `{}`. Resolve manually.",
                                dest.display(),
                                original_path.display()
                            );
                        }
                    } else {
                        println!("ERROR: Destination `{}` exists but it's not a symlink. Resolve manually", dest.display());
                    }
                }
                _ => {
                    println!(
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
