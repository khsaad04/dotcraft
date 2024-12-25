mod cli;
mod colors;

use clap::Parser;
use color_eyre::eyre::{self, Context};
use core::panic;
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
    target: PathBuf,
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
    match &cli.command {
        Some(cli::Commands::Sync { force }) => {
            sync_files(&manifest.files, &manifest.config, force)?;
        }
        Some(cli::Commands::Link { force }) => {
            link_files(&manifest.files, force)?;
        }
        Some(cli::Commands::Generate) => {
            if has_templates(&manifest) {
                generate_templates(&manifest.files, &manifest.config)?;
            } else {
                println!("WARNING: There are no templates. Skipping template generation");
            }
        }
        None => {}
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

fn sync_files(
    files: &HashMap<String, File>,
    config: &HashMap<String, String>,
    force_flag: &bool,
) -> eyre::Result<()> {
    for (_, file) in files.iter() {
        let dest_path = file.dest.clone().unwrap_or("".into());
        let home_dir = std::env::var("HOME")?;
        let dest = PathBuf::from(home_dir).join(dest_path).join(&file.target);
        if !dest.parent().unwrap().exists() {
            fs::create_dir_all(dest.parent().unwrap())?;
        }
        let target = file
            .target
            .canonicalize()
            .context(format!("Target {:?} not found", &file.target))?;
        if let Some(template) = &file.template {
            generate_template(config, template, &target)?;
        }
        symlink_dir_all(&target, &dest, force_flag)?;
    }
    Ok(())
}

fn link_files(files: &HashMap<String, File>, force_flag: &bool) -> eyre::Result<()> {
    for (_, file) in files.iter() {
        let dest_path = file.dest.clone().unwrap_or("".into());
        let home_dir = std::env::var("HOME")?;
        let dest = PathBuf::from(home_dir).join(dest_path).join(&file.target);
        if !dest.parent().unwrap().exists() {
            fs::create_dir_all(dest.parent().unwrap())?;
        }
        let target = file
            .target
            .canonicalize()
            .context(format!("Target {:?} not found", &file.target))?;
        symlink_dir_all(&target, &dest, force_flag)?;
    }
    Ok(())
}

fn generate_templates(
    files: &HashMap<String, File>,
    config: &HashMap<String, String>,
) -> eyre::Result<()> {
    for (_, file) in files.iter() {
        let target = file
            .target
            .canonicalize()
            .context(format!("Target {:?} not found", &file.target))?;
        if let Some(template) = &file.template {
            generate_template(config, template, &target)?;
        }
    }
    Ok(())
}

fn generate_template(
    config: &HashMap<String, String>,
    template: &Path,
    target: &Path,
) -> eyre::Result<()> {
    let template = template
        .canonicalize()
        .context(format!("Template {:?} not found", &template))?;
    let template_path = template.to_str().unwrap();
    let data = fs::read_to_string(template_path)
        .context(format!("Failed to parse template {:?}", template_path))?;

    let mut engine = upon::Engine::new();
    engine
        .add_template(template_path, &data)
        .context("Failed to add template to template engine")?;
    let rendered = engine
        .template(template_path)
        .render(config)
        .to_string()
        .context(format!("Failed to render template {:?}", template_path))?;
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
    if target.exists() {
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
                        eprintln!("ERROR: Destination {:?} exists but it's not a symlink. Please resolve manually", dest);
                    }
                }
                _ => {
                    eprintln!(
                        "ERROR: Failed to symlink {:?} -> {:?}. {}",
                        target, dest, err
                    );
                }
            },
        }
    } else {
        eprintln!("ERROR: Target {:?} not found", target);
    }
    Ok(())
}
