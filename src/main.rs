use material_colors::{
    image::{FilterType, ImageReader},
    theme::ThemeBuilder,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, create_dir_all},
    io::{self, ErrorKind},
    os::unix::fs::symlink,
    path::PathBuf,
    str::FromStr,
};
use tinytemplate::TinyTemplate;

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    config: HashMap<String, String>,
    files: HashMap<String, File>,
}

#[derive(Debug, Deserialize, Serialize)]
struct File {
    target: PathBuf,
    dest: PathBuf,
    template: Option<PathBuf>,
}

fn main() -> io::Result<()> {
    let manifest_path = PathBuf::from_str("Manifest.toml").unwrap();
    if !manifest_path.exists() {
        eprintln!("ERROR: Manifest.toml not found");
    }
    let mut manifest: Manifest = toml::from_str(&fs::read_to_string(manifest_path)?)
        .expect("ERROR: Failed to parse Manifest.toml");

    let wallpaper = manifest
        .config
        .get("wallpaper")
        .expect("ERROR: config wallpaper not found");
    let wp_path = PathBuf::from_str(wallpaper).unwrap();
    if wp_path.exists() {
        let mut image = ImageReader::open(wallpaper).unwrap();
        image.resize(128, 128, FilterType::Lanczos3);
        let theme = ThemeBuilder::with_source(ImageReader::extract_color(&image)).build();

        for (k, v) in theme.schemes.dark.into_iter() {
            manifest.config.insert(k, v.to_hex());
        }
    } else {
        eprintln!("ERROR: Wallpaper `{}` not found", wp_path.to_str().unwrap());
    }

    for (_, file) in manifest.files.into_iter() {
        if let Some(template) = file.template {
            parse_file(&manifest.config, template, &file.target)?;
        }
        symlink_file(&file.target, &file.dest)?;
    }

    Ok(())
}

fn parse_file(
    config: &HashMap<String, String>,
    template: PathBuf,
    target: &PathBuf,
) -> io::Result<()> {
    let mut engine = TinyTemplate::new();

    let template_path = template.to_str().unwrap();
    let data = fs::read_to_string(template_path).expect("Failed to read template file");
    engine
        .add_template(template_path, &data)
        .expect("Failed to add template to template engine");

    let rendered = engine
        .render(template_path, &config)
        .expect("Failed to render the template");
    fs::write(target, rendered)?;
    Ok(())
}

fn symlink_file(target: &PathBuf, dest: &PathBuf) -> io::Result<()> {
    let target_path = target.to_str().unwrap();
    let dest_path = dest.to_str().unwrap();
    if target.exists() {
        match symlink(target, dest) {
            Ok(()) => {
                println!("INFO: Symlinked `{}` to `{}`", target_path, dest_path);
            }
            Err(err) => match err.kind() {
                ErrorKind::AlreadyExists => {
                    eprintln!(
                        "ERROR: Destination `{}` already exists, resolve it manually",
                        dest_path
                    );
                }
                ErrorKind::NotFound => {
                    let dirpath = dest.parent().unwrap();
                    println!("INFO: Creating directory `{}`", dirpath.to_str().unwrap());
                    create_dir_all(dirpath)?;
                    symlink_file(target, dest)?;
                }
                _ => {
                    eprintln!(
                        "ERROR: Could not symlink `{}` to `{}`. {}",
                        target_path, dest_path, err
                    );
                }
            },
        }
    } else {
        eprintln!("ERROR: Target `{}` not found", target_path);
    }
    Ok(())
}
