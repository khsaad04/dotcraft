use crate::Config;
use color_eyre::eyre;
use material_colors::{
    image::{FilterType, ImageReader},
    theme::ThemeBuilder,
};
use std::{collections::HashMap, path::PathBuf};

pub fn generate_material_colors(
    wp_path: PathBuf,
    dark: bool,
    config: &mut Config,
) -> eyre::Result<()> {
    let mut image = ImageReader::open(wp_path)?;
    image.resize(128, 128, FilterType::Lanczos3);
    let theme = ThemeBuilder::with_source(ImageReader::extract_color(&image)).build();

    config.insert("source_color".to_string(), theme.source.to_hex());

    if dark {
        for (k, v) in theme.schemes.dark.into_iter() {
            config.insert(k, v.to_hex());
        }
    } else {
        for (k, v) in theme.schemes.light.into_iter() {
            config.insert(k, v.to_hex());
        }
    }
    generate_base16_colors(config, &theme.source.to_hex())?;
    Ok(())
}

pub fn generate_base16_colors(
    config: &mut HashMap<String, String>,
    source_color: &str,
) -> eyre::Result<()> {
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

fn blend_color(first: &str, second: &str, weight: f32) -> eyre::Result<String> {
    let w = weight * 2.0 - 1.0;
    let w1 = (w / 2.0) + 0.5;
    let w2 = 1.0 - w1;
    let first_r = u32::from_str_radix(&first[..2], 16)?;
    let first_g = u32::from_str_radix(&first[2..4], 16)?;
    let first_b = u32::from_str_radix(&first[4..6], 16)?;
    let second_r = u32::from_str_radix(&second[..2], 16)?;
    let second_g = u32::from_str_radix(&second[2..4], 16)?;
    let second_b = u32::from_str_radix(&second[4..6], 16)?;
    let r = (first_r as f32 * w1 + second_r as f32 * w2).clamp(16.0, 255.0) as u8;
    let g = (first_g as f32 * w1 + second_g as f32 * w2).clamp(16.0, 255.0) as u8;
    let b = (first_b as f32 * w1 + second_b as f32 * w2).clamp(16.0, 255.0) as u8;
    Ok(format!("{:x}{:x}{:x}", r, g, b).to_string())
}
