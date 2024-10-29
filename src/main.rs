use color_eyre::eyre::{Context, Result};
use material_colors::{
    image::{FilterType, ImageReader},
    theme::ThemeBuilder,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, create_dir_all},
    io::ErrorKind,
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

fn main() -> Result<()> {
    color_eyre::install()?;

    let manifest_path = PathBuf::from_str("Manifest.toml")?.canonicalize()?;
    if !manifest_path.exists() {
        eprintln!("ERROR: Manifest.toml not found");
    }
    let mut manifest: Manifest = toml::from_str(&fs::read_to_string(manifest_path)?)
        .context("ERROR: Failed to parse Manifest.toml")?;

    let wallpaper = manifest
        .config
        .get("wallpaper")
        .expect("ERROR: config wallpaper not found");
    let wp_path = PathBuf::from_str(wallpaper)?;
    if !wp_path.exists() {
        eprintln!("ERROR: Wallpaper `{:?}` not found", wp_path);
    }
    let mut image = ImageReader::open(wallpaper)?;
    image.resize(128, 128, FilterType::Lanczos3);
    let theme = ThemeBuilder::with_source(ImageReader::extract_color(&image)).build();

    for (k, v) in theme.schemes.dark.into_iter() {
        manifest.config.insert(k, v.to_hex());
    }

    generate_base16_colors(&mut manifest.config, &theme.source.to_hex())?;
    parse_files(&manifest.files, &manifest.config)?;
    Ok(())
}

fn parse_files(files: &HashMap<String, File>, config: &HashMap<String, String>) -> Result<()> {
    for (_, file) in files.iter() {
        let dest_path = file.dest.clone().unwrap_or("".into());
        let home_dir = std::env::var("HOME")?;
        let dest = PathBuf::from(home_dir).join(dest_path).join(&file.target);
        if !dest.parent().unwrap().exists() {
            create_dir_all(dest.parent().unwrap())?;
        }
        let target = file.target.canonicalize()?;
        if let Some(template) = &file.template {
            generate_template(config, template, &target)?;
        }
        symlink_dir_all(&target, &dest)?;
    }
    Ok(())
}

fn generate_template(
    config: &HashMap<String, String>,
    template: &Path,
    target: &Path,
) -> Result<()> {
    let template = template.canonicalize()?;
    let template_path = template.to_str().unwrap();
    let data = fs::read_to_string(template_path).context("Failed to parse template file")?;

    let mut engine = upon::Engine::new();
    engine
        .add_template(template_path, &data)
        .context("Failed to add template to template engine")?;
    let rendered = engine.template(template_path).render(config).to_string()?;
    fs::write(target, rendered)?;
    println!("INFO: Generated `{:?}` template", template);
    Ok(())
}

fn symlink_dir_all(target: &Path, dest: &Path) -> Result<()> {
    if target.is_dir() {
        for entry in fs::read_dir(target)? {
            let entry = entry?;
            let dest = &dest.join(entry.path().file_name().unwrap());
            if !dest.parent().unwrap().exists() {
                create_dir_all(dest.parent().unwrap())?;
            }
            symlink_dir_all(&entry.path(), dest)?;
        }
    } else {
        symlink_file(target, dest)?;
    }
    Ok(())
}

fn symlink_file(target: &Path, dest: &Path) -> Result<()> {
    if target.exists() {
        match symlink(target, dest) {
            Ok(()) => {
                println!("INFO: Symlinked `{:?}` -> `{:?}`", target, dest);
            }
            Err(err) => match err.kind() {
                ErrorKind::AlreadyExists => {
                    println!("WARNING: Destination `{:?}` already symlinked", dest);
                }
                _ => {
                    eprintln!(
                        "ERROR: Failed to symlink `{:?}` to `{:?}`. {}",
                        target, dest, err
                    );
                }
            },
        }
    } else {
        eprintln!("ERROR: Target `{:?}` not found", target);
    }
    Ok(())
}

fn generate_base16_colors(config: &mut HashMap<String, String>, source_color: &str) -> Result<()> {
    let base16: [(&str, &str); 16] = [
        ("base0", "000000"),
        ("base1", "ff0000"),
        ("base2", "00ff00"),
        ("base3", "ffff00"),
        ("base4", "0000ff"),
        ("base5", "ff00ff"),
        ("base6", "00ffff"),
        ("base7", "ffffff"),
        ("base8", "000000"),
        ("base9", "ff0000"),
        ("base10", "00ff00"),
        ("base11", "ffff00"),
        ("base12", "0000ff"),
        ("base13", "ff00ff"),
        ("base14", "00ffff"),
        ("base15", "ffffff"),
    ];
    for (name, value) in base16.into_iter() {
        let mut weight: f32 = 0.3;
        if name[4..].parse::<usize>().unwrap() > 7 {
            weight = 0.5;
        }
        let new_color = blend_color(value, source_color, weight)?;
        config.insert(name.to_string(), new_color);
    }
    Ok(())
}

fn blend_color(first: &str, second: &str, weight: f32) -> Result<String> {
    let w1 = weight;
    let w2 = 1.0 - w1;
    let first_r = i64::from_str_radix(&first[..2], 16)?;
    let first_g = i64::from_str_radix(&first[2..4], 16)?;
    let first_b = i64::from_str_radix(&first[4..6], 16)?;
    let second_r = i64::from_str_radix(&second[..2], 16)?;
    let second_g = i64::from_str_radix(&second[2..4], 16)?;
    let second_b = i64::from_str_radix(&second[4..6], 16)?;
    let r = (first_r as f32 * w1 + second_r as f32 * w2) as i64;
    let g = (first_g as f32 * w1 + second_g as f32 * w2) as i64;
    let b = (first_b as f32 * w1 + second_b as f32 * w2) as i64;
    Ok(format!("{:x}{:x}{:x}", r, g, b).to_string())
}
