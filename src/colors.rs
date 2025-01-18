use crate::{Result, VarMap};

use material_colors::{
    color::Argb,
    image::{FilterType, ImageReader},
    theme::ThemeBuilder,
};
use std::{collections::HashMap, path::Path};

pub fn generate_material_colors(wp_path: &Path, theme: &str, config: &mut VarMap) -> Result<()> {
    let mut image = ImageReader::open(wp_path).map_err(|err| {
        format!(
            "could not read image {path}: {err}",
            path = &wp_path.display()
        )
    })?;
    image.resize(128, 128, FilterType::Lanczos3);
    let color_palette = ThemeBuilder::with_source(ImageReader::extract_color(&image)).build();

    config.insert("source_color".to_string(), color_palette.source.to_hex());

    if theme == "dark" {
        for (k, v) in color_palette.schemes.dark.into_iter() {
            config.insert(k, v.to_hex());
        }
    } else if theme == "light" {
        for (k, v) in color_palette.schemes.light.into_iter() {
            config.insert(k, v.to_hex());
        }
    } else {
        return Err(format!("invalid theme {theme}").into());
    }
    generate_base16_colors(config, &color_palette.source);
    config.insert("theme".to_string(), theme.to_string());
    Ok(())
}

pub fn generate_base16_colors(config: &mut HashMap<String, String>, source_color: &Argb) {
    let base16: [(&str, &Argb); 16] = [
        ("base0", &Argb::new(255, 0, 0, 0)),
        ("base1", &Argb::new(255, 128, 0, 0)),
        ("base2", &Argb::new(255, 0, 128, 0)),
        ("base3", &Argb::new(255, 128, 128, 0)),
        ("base4", &Argb::new(255, 0, 0, 128)),
        ("base5", &Argb::new(255, 128, 0, 128)),
        ("base6", &Argb::new(255, 0, 128, 128)),
        ("base7", &Argb::new(255, 192, 192, 192)),
        ("base8", &Argb::new(255, 128, 128, 128)),
        ("base9", &Argb::new(255, 255, 0, 0)),
        ("base10", &Argb::new(255, 0, 255, 0)),
        ("base11", &Argb::new(255, 255, 255, 0)),
        ("base12", &Argb::new(255, 0, 0, 255)),
        ("base13", &Argb::new(255, 255, 0, 255)),
        ("base14", &Argb::new(255, 0, 255, 255)),
        ("base15", &Argb::new(255, 255, 255, 255)),
    ];
    for (name, value) in base16.into_iter() {
        let new_color = blend_color(value, source_color);
        config.insert(name.to_string(), new_color.to_hex());
    }
}

fn blend_color(first: &Argb, second: &Argb) -> Argb {
    let r = (first.red as f32 * 0.5 + second.red as f32 * 0.5) as u8;
    let g = (first.green as f32 * 0.5 + second.green as f32 * 0.5) as u8;
    let b = (first.blue as f32 * 0.5 + second.blue as f32 * 0.5) as u8;
    Argb::new(255, r, g, b)
}
