use crate::error::Result;
use crate::ContextMap;

use material_colors::{color::Argb, dynamic_color::Variant, theme::ThemeBuilder};
use quantette::{image, PalettePipeline};
use std::{collections::HashMap, path::Path};

pub fn generate_material_colors(
    wp_path: &Path,
    theme: &str,
    variant: &str,
    context: &mut ContextMap,
) -> Result<()> {
    let img = image::open(wp_path)
        .map_err(|err| format!("Could not load image {}: {err}", wp_path.display()))?
        .into_rgb8();
    let mut pipeline = PalettePipeline::try_from(&img).map_err(|err| {
        format!(
            "Could not color quantize image {}: {err}",
            wp_path.display()
        )
    })?;
    let quantized_palette = pipeline.palette_size(1).palette_par();
    let color = Argb::new(
        255,
        quantized_palette[0].red,
        quantized_palette[0].green,
        quantized_palette[0].blue,
    );

    let variant = match variant {
        "monochrome" => Variant::Monochrome,
        "neutral" => Variant::Neutral,
        "tonal_spot" => Variant::TonalSpot,
        "vibrant" => Variant::Vibrant,
        "expressive" => Variant::Expressive,
        "fidelity" => Variant::Fidelity,
        "content" => Variant::Content,
        "rainbow" => Variant::Rainbow,
        "fruit_salad" => Variant::FruitSalad,
        _ => return Err(format!("invalid variant {variant}\nPossible values: \"monochrome\", \"neutral\", \"tonal_spot\", \"vibrant\", \"expressive\", \"fidelity\", \"content\", \"rainbow\", \"fruit_salad\"").into()),
    };

    let color_palette = ThemeBuilder::with_source(color).variant(variant).build();

    context.insert("source_color".to_string(), color_palette.source.to_hex());

    match theme {
        "dark" => {
            for (k, v) in color_palette.schemes.dark.into_iter() {
                context.insert(k, v.to_hex());
            }
        }
        "light" => {
            for (k, v) in color_palette.schemes.light.into_iter() {
                context.insert(k, v.to_hex());
            }
        }
        _ => {
            return Err(
                format!("invalid theme {theme}\nPossible values: \"dark\", \"light\"").into(),
            )
        }
    }

    generate_base16_colors(context, &color);
    context.insert("theme".to_string(), theme.to_string());
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

fn blend_color(a: &Argb, b: &Argb) -> Argb {
    let r = a.red / 2 + b.red / 2;
    let g = a.green / 2 + b.green / 2;
    let b = a.blue / 2 + b.blue / 2;
    Argb::new(255, r, g, b)
}
