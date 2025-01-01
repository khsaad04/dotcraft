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
    str::FromStr,
};

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    config: HashMap<String, String>,
    files: HashMap<String, File>,
}

#[derive(Debug, Deserialize, Serialize)]
struct File {
    target: String,
    dest: Option<String>,
    template: Option<String>,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let cli = cli::Cli::parse();

    // Parse Manifest file
    let manifest_path = cli
        .manifest
        .canonicalize()
        .context("Manifest.toml not found")?;
    std::env::set_current_dir(manifest_path.parent().unwrap())?;
    let mut manifest: Manifest = toml::from_str(
        &fs::read_to_string(manifest_path).context("Failed to read file Manifest.toml")?,
    )
    .context("ERROR: Failed to parse Manifest.toml")?;

    // Generate color scheme from wallpaper
    if let Some(wallpaper) = manifest.config.get("wallpaper") {
        let wp_path = PathBuf::from_str(wallpaper)?
            .canonicalize()
            .context(format!("Wallpaper {:?} not found", wallpaper))?;
        manifest.config.insert(
            "wallpaper".to_string(),
            wp_path.to_str().unwrap().to_string(),
        );
        colors::generate_material_colors(wp_path, &mut manifest)?;
    } else if has_templates(&manifest) {
        panic!("`wallpaper` is not set. Needed for color scheme generation");
    } else {
        println!("WARNING: Skipping color scheme generation.");
    }

    // Execute commands
    for (_, file) in manifest.files.iter() {
        let globbed_path = glob(&file.target).context(format!(
            "Failed to parse target `{}`. Invalid glob pattern",
            &file.target
        ))?;
        for entry in globbed_path {
            let entry = entry?;
            let target_path = PathBuf::from(&entry)
                .canonicalize()
                .context(format!("Target {:?} not found", &entry))?;
            let home_dir = PathBuf::from(std::env::var("HOME")?);
            let dest_path = if let Some(dest) = &file.dest {
                let dest = PathBuf::from(dest);
                if dest.is_dir() {
                    home_dir.join(dest).join(entry.iter().last().unwrap())
                } else {
                    home_dir.join(dest)
                }
            } else {
                home_dir.join(&entry)
            };
            if !dest_path.parent().unwrap().exists() {
                fs::create_dir_all(dest_path.parent().unwrap())?;
            }
            match &cli.command {
                Some(cli::Commands::Sync { force }) => {
                    if let Some(template_path) = &file.template {
                        generate_template(
                            &manifest.config,
                            &PathBuf::from(template_path)
                                .canonicalize()
                                .context(format!("Template {:?} not found", &template_path))?,
                            &target_path,
                        )?;
                    }
                    symlink_dir_all(&target_path, &dest_path, force)?;
                }
                Some(cli::Commands::Link { force }) => {
                    symlink_dir_all(&target_path, &dest_path, force)?;
                }
                Some(cli::Commands::Generate) => {
                    if let Some(template_path) = &file.template {
                        generate_template(
                            &manifest.config,
                            &PathBuf::from(template_path)
                                .canonicalize()
                                .context(format!("Template {:?} not found", &template_path))?,
                            &target_path,
                        )?;
                    }
                }
                None => {
                    unreachable!()
                }
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
    config: &HashMap<String, String>,
    template: &Path,
    target: &Path,
) -> eyre::Result<()> {
    let data =
        fs::read_to_string(template).context(format!("Failed to parse template {:?}", template))?;

    let mut engine = upon::Engine::new();
    engine
        .add_template(template.to_str().unwrap(), &data)
        .context("Failed to add template to template engine")?;
    let rendered = engine
        .template(template.to_str().unwrap())
        .render(config)
        .to_string()
        .context(format!("Failed to render template {:?}", template))?;

    fs::write(target, rendered)?;
    println!("INFO: Generated {:?} template", template);
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
            println!("INFO: Symlinked {:?} -> {:?}", target, dest);
        }
        Err(err) => match err.kind() {
            io::ErrorKind::AlreadyExists => {
                if *force_flag {
                    std::fs::remove_file(dest)?;
                    println!("WARNING: Destination {:?} already exists. Removing", dest);
                    symlink(target, dest)?;
                    println!("INFO: Symlinked {:?} -> {:?}", target, dest);
                    return Ok(());
                }
                if dest.is_symlink() {
                    println!(
                        "WARNING: Destination {:?} already symlinked. Skipping",
                        dest
                    );
                } else {
                    println!("ERROR: Destination {:?} exists but it's not a symlink. Please resolve manually", dest);
                }
            }
            _ => {
                println!(
                    "ERROR: Failed to symlink {:?} -> {:?}. {}",
                    target, dest, err
                );
            }
        },
    }
    Ok(())
}
