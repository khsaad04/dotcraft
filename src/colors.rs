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
    Ok(())
}

pub fn generate_base16_colors(config: &mut HashMap<String, String>, source_color: &Argb) {
    let base16: [(&str, &Argb); 16] = [
        ("base0", &Argb::new(255, 0, 0, 0)),
        ("base1", &Argb::new(255, 255, 0, 0)),
        ("base2", &Argb::new(255, 0, 255, 0)),
        ("base3", &Argb::new(255, 255, 255, 0)),
        ("base4", &Argb::new(255, 0, 0, 255)),
        ("base5", &Argb::new(255, 255, 0, 255)),
        ("base6", &Argb::new(255, 0, 255, 255)),
        ("base7", &Argb::new(255, 255, 255, 255)),
        ("base8", &Argb::new(255, 0, 0, 0)),
        ("base9", &Argb::new(255, 255, 0, 0)),
        ("base10", &Argb::new(255, 0, 255, 0)),
        ("base11", &Argb::new(255, 255, 255, 0)),
        ("base12", &Argb::new(255, 0, 0, 255)),
        ("base13", &Argb::new(255, 255, 0, 255)),
        ("base14", &Argb::new(255, 0, 255, 255)),
        ("base15", &Argb::new(255, 255, 255, 255)),
    ];
    for (i, (name, value)) in base16.into_iter().enumerate() {
        let mut weight = 0.3;
        if i > 7 {
            weight = 0.5;
        }
        let new_color = blend_color(value, source_color, weight);
        config.insert(name.to_string(), new_color.to_hex());
    }
}

fn blend_color(first: &Argb, second: &Argb, weight: f32) -> Argb {
    let w = weight * 2.0 - 1.0;
    let w1 = (w / 2.0) + 0.5;
    let w2 = 1.0 - w1;
    let r = (first.red as f32 * w1 + second.red as f32 * w2) as u8;
    let g = (first.green as f32 * w1 + second.green as f32 * w2) as u8;
    let b = (first.blue as f32 * w1 + second.blue as f32 * w2) as u8;
    Argb::new(255, r, g, b)
}
