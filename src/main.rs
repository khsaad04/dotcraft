mod cli;
mod colors;

use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fs,
    io::{self, Write},
    os::unix::fs::symlink as symlink_unix,
    path, process,
};

#[macro_use]
mod helper;
use helper::*;

#[derive(Debug, Deserialize)]
struct Manifest {
    options: ManifestOpt,
    variables: Option<HashMap<String, String>>,
    entries: HashMap<String, Vec<Entry>>,
}

#[derive(Debug, Deserialize)]
struct ManifestOpt {
    wallpaper: Option<path::PathBuf>,
    #[serde(default = "default_theme_option")]
    theme: String,
    #[serde(default = "default_variant_option")]
    variant: String,
}

fn default_theme_option() -> String {
    "dark".to_string()
}

fn default_variant_option() -> String {
    "tonal_spot".to_string()
}

#[derive(Debug, Deserialize)]
struct Entry {
    target: Option<path::PathBuf>,
    dest: path::PathBuf,
    template: Option<path::PathBuf>,
    #[serde(default = "default_recursive_option")]
    recursive: bool,
    pre_hooks: Option<Vec<String>>,
    post_hooks: Option<Vec<String>>,
}

const fn default_recursive_option() -> bool {
    false
}

type TemplateContext = HashMap<String, String>;

fn init_template_context(context: &mut TemplateContext, manifest: &Manifest) -> Result<()> {
    if let Some(wallpaper) = &manifest.options.wallpaper {
        let wallpaper_path = resolve_home_dir(wallpaper)?
            .canonicalize()
            .map_err(|err| format!("could not find {}: {err}", wallpaper.display()))?;
        context.insert(
            "wallpaper".to_string(),
            wallpaper_path.display().to_string(),
        );
        colors::generate_material_colors(
            &wallpaper_path,
            &manifest.options.theme,
            &manifest.options.variant,
            context,
        )?;
    } else if has_templates(manifest) {
        return Err("could not generate color palette: wallpaper is not set."
            .to_string()
            .into());
    } else {
        log!(Warning, "Skipping color scheme generation.");
    }

    if let Some(vars) = &manifest.variables {
        for (k, v) in vars {
            context.insert(k.to_string(), v.to_string());
        }
    }
    Ok(())
}

impl TryFrom<&path::Path> for Manifest {
    type Error = Error;
    fn try_from(value: &path::Path) -> std::result::Result<Self, Self::Error> {
        let path = value
            .canonicalize()
            .map_err(|err| format!("invalid path {}: {err}", value.display()))?;
        let parent_dir = path
            .parent()
            .ok_or(format!("could not access parent dir of {}", path.display()))?;
        env::set_current_dir(parent_dir).map_err(|err| {
            format!(
                "could not change directory to {}: {err}",
                parent_dir.display()
            )
        })?;
        let manifest: Manifest = toml::from_str(
            &fs::read_to_string(&path)
                .map_err(|err| format!("could not read file {}: {err}", path.display()))?,
        )
        .map_err(|err| format!("could not parse toml {}: {err}", path.display()))?;
        Ok(manifest)
    }
}

fn main() {
    if let Err(err) = entrypoint() {
        log!(Error, "{err}");
        process::exit(1);
    }
}

fn entrypoint() -> Result<()> {
    let args = cli::Cli::try_parse()?;

    let manifest = Manifest::try_from(args.manifest_path.as_path())?;
    let mut context: TemplateContext = HashMap::new();

    let mut template_engine = upon::Engine::new();

    match args.subcommand {
        cli::SubCommand::Sync {
            force,
            dry,
            ref name,
        } => {
            if dry {
                log!(Warning, "Performing a dry run.");
            }
            if let Some(name) = name {
                if let Some(entries) = manifest.entries.get(name) {
                    for entry in entries {
                        if let Some(pre_hook) = &entry.pre_hooks {
                            for cmd in pre_hook.iter() {
                                log!(Info, "Executing pre-hook in {}: {}", name, cmd);
                                if !dry {
                                    execute_hook(cmd)?;
                                }
                            }
                        }

                        if let Some(target) = &entry.target {
                            symlink_dir_all(target, &entry.dest, force, dry, entry.recursive)
                                .map_err(|err| {
                                    format!(
                                        "something went wrong while symlinking {name}:\n    {err}"
                                    )
                                })?;
                        }

                        if let Some(template) = &entry.template {
                            init_template_context(&mut context, &manifest)?;
                            generate_template(
                                &entry.dest,
                                template,
                                &context,
                                &mut template_engine,
                                dry,
                            )
                            .map_err(|err| {
                                format!("something went wrong while generating {name}:\n    {err}")
                            })?;
                        }

                        if let Some(post_hook) = &entry.post_hooks {
                            for cmd in post_hook.iter() {
                                log!(Info, "Executing post-hook in {}: {}", name, cmd);
                                if !dry {
                                    execute_hook(cmd)?;
                                }
                            }
                        }
                    }
                } else {
                    return Err(format!("could not find {}", &name).into());
                }
            } else {
                if has_templates(&manifest) {
                    init_template_context(&mut context, &manifest)?;
                }
                for (name, entries) in manifest.entries.iter() {
                    for entry in entries {
                        if let Some(pre_hook) = &entry.pre_hooks {
                            for cmd in pre_hook.iter() {
                                log!(Info, "Executing pre-hook in {}: {}", name, cmd);
                                if !dry {
                                    execute_hook(cmd)?;
                                }
                            }
                        }

                        if let Some(target) = &entry.target {
                            symlink_dir_all(target, &entry.dest, force, dry, entry.recursive)
                                .map_err(|err| {
                                    format!(
                                        "something went wrong while symlinking {name}:\n    {err}"
                                    )
                                })?;
                        }

                        if let Some(template) = &entry.template {
                            generate_template(
                                &entry.dest,
                                template,
                                &context,
                                &mut template_engine,
                                dry,
                            )
                            .map_err(|err| {
                                format!("something went wrong while generating {name}:\n    {err}")
                            })?;
                        }

                        if let Some(post_hook) = &entry.post_hooks {
                            for cmd in post_hook.iter() {
                                log!(Info, "Executing post-hook in {}: {}", name, cmd);
                                if !dry {
                                    execute_hook(cmd)?;
                                }
                            }
                        }
                    }
                }
            }
        }
        cli::SubCommand::Link {
            force,
            dry,
            ref name,
        } => {
            if dry {
                log!(Warning, "Performing a dry run.");
            }
            if let Some(name) = name {
                if let Some(entries) = manifest.entries.get(name) {
                    for entry in entries {
                        if let Some(target) = &entry.target {
                            symlink_dir_all(target, &entry.dest, force, dry, entry.recursive)
                                .map_err(|err| {
                                    format!(
                                        "something went wrong while symlinking {name}:\n    {err}"
                                    )
                                })?;
                        }
                    }
                } else {
                    return Err(format!("could not find {}", &name).into());
                }
            } else {
                for (name, entries) in manifest.entries.iter() {
                    for entry in entries {
                        if let Some(target) = &entry.target {
                            symlink_dir_all(target, &entry.dest, force, dry, entry.recursive)
                                .map_err(|err| {
                                    format!(
                                        "something went wrong while symlinking {name}:\n    {err}"
                                    )
                                })?;
                        }
                    }
                }
            }
        }
        cli::SubCommand::Generate { dry, ref name } => {
            if dry {
                log!(Warning, "Performing a dry run.");
            }
            if let Some(name) = name {
                if let Some(entries) = manifest.entries.get(name) {
                    for entry in entries {
                        if let Some(template) = &entry.template {
                            init_template_context(&mut context, &manifest)?;
                            generate_template(
                                &entry.dest,
                                template,
                                &context,
                                &mut template_engine,
                                dry,
                            )
                            .map_err(|err| {
                                format!("something went wrong while generating {name}:\n    {err}")
                            })?;
                        }
                    }
                } else {
                    return Err(format!("could not find {}", &name).into());
                }
            } else {
                if has_templates(&manifest) {
                    init_template_context(&mut context, &manifest)?;
                }
                for (name, entries) in manifest.entries.iter() {
                    for entry in entries {
                        if let Some(template) = &entry.template {
                            generate_template(
                                &entry.dest,
                                template,
                                &context,
                                &mut template_engine,
                                dry,
                            )
                            .map_err(|err| {
                                format!("something went wrong while generating {name}:\n    {err}")
                            })?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn has_templates(manifest: &Manifest) -> bool {
    for (_, entries) in manifest.entries.iter() {
        for entry in entries {
            if entry.template.is_some() {
                return true;
            }
        }
    }
    false
}

fn resolve_home_dir(the_path: impl AsRef<path::Path>) -> Result<path::PathBuf> {
    let the_path = the_path.as_ref();
    let home_dir =
        env::var("HOME").map_err(|err| format!("could not find home directory: {err}"))?;

    if let Ok(stripped_path) = the_path.strip_prefix("~") {
        Ok(path::PathBuf::from(home_dir).join(stripped_path))
    } else {
        Ok(the_path.to_path_buf())
    }
}

fn symlink_dir_all(
    target: impl AsRef<path::Path>,
    dest: impl AsRef<path::Path>,
    force: bool,
    dry: bool,
    recursive: bool,
) -> Result<()> {
    let target = resolve_home_dir(&target)?
        .canonicalize()
        .map_err(|err| format!("could not find {}: {err}", target.as_ref().display()))?;
    let dest = resolve_home_dir(dest)?;

    if target.is_dir() && recursive {
        for entry in fs::read_dir(target)? {
            let entry = entry?;
            let dest = &dest.join(entry.path().file_name().ok_or(format!(
                "could not extract file_name of {}",
                entry.path().display()
            ))?);
            let dest_parent_dir = dest
                .parent()
                .ok_or(format!("could not access parent dir of {}", dest.display()))?;
            if !dest_parent_dir.exists() && !dry {
                fs::create_dir_all(dest_parent_dir).map_err(|err| {
                    format!("could not create dir {}: {err}", dest_parent_dir.display())
                })?;
            }
            symlink_dir_all(entry.path(), dest, force, dry, recursive)?;
        }
    } else {
        symlink_file(&target, &dest, force, dry)?;
    }
    Ok(())
}

fn symlink_file(
    target: impl AsRef<path::Path>,
    dest: impl AsRef<path::Path>,
    force: bool,
    dry: bool,
) -> Result<()> {
    let target = target.as_ref();
    let dest = dest.as_ref();

    if dest.exists() {
        if force {
            log!(
                Warning,
                "Destination {} already exists. Removing",
                dest.display()
            );
            if !dry {
                fs::remove_file(dest)
                    .map_err(|err| format!("could not remove file {}: {err}", dest.display()))?;
            }
        } else if dest.is_symlink() {
            let symlink_origin = dest.canonicalize()?;
            if target.canonicalize()? == symlink_origin {
                log!(Info, "Symlink up-to-date: {}", dest.display());
            } else {
                log!(
                    Warning,
                    "Destination {} is symlinked to {}. Resolve manually.",
                    dest.display(),
                    symlink_origin.display()
                );
            }
            return Ok(());
        } else {
            log!(
                Warning,
                "Destination {} exists but it's not a symlink. Resolve manually",
                dest.display()
            );
            return Ok(());
        }
    } else if dest.is_symlink() {
        log!(
            Warning,
            "Destination {} is a broken symlink. Ignoring",
            dest.display()
        );
        if !dry {
            fs::remove_file(dest)
                .map_err(|err| format!("could not remove file {}: {err}", dest.display()))?;
        }
    } else if !dry {
        let dest_parent = dest
            .parent()
            .ok_or(format!("could not access parent dir of {}", dest.display()))?;
        fs::create_dir_all(dest_parent)
            .map_err(|err| format!("could not create dir {}: {err}", dest_parent.display()))?;
    }

    if !dry {
        symlink_unix(target, dest).map_err(|err| {
            format!(
                "could not symlink {} to {}: {err}",
                target.display(),
                dest.display()
            )
        })?;
    }

    log!(Info, "Symlinked {} -> {}", target.display(), dest.display());
    Ok(())
}

fn generate_template(
    dest: impl AsRef<path::Path>,
    template: impl AsRef<path::Path>,
    context: &TemplateContext,
    template_engine: &mut upon::Engine,
    dry: bool,
) -> Result<()> {
    let template = resolve_home_dir(template.as_ref())?
        .canonicalize()
        .map_err(|err| format!("could not find {}: {err}", template.as_ref().display()))?;
    let dest = resolve_home_dir(dest.as_ref())?;

    let data = fs::read_to_string(&template)
        .map_err(|err| format!("could not read file {}: {err}", template.display()))?;

    let rendered = template_engine
        .compile(&data)
        .map_err(|err| format!("could not compile template {}: {err}", template.display()))?
        .render(template_engine, context)
        .to_string()
        .map_err(|err| format!("could not render template {}: {err}", template.display()))?;

    let dest_parent = dest
        .parent()
        .ok_or(format!("could not access parent dir of {}", dest.display()))?;

    if !dry {
        if !dest_parent.exists() {
            fs::create_dir_all(dest_parent)?;
        }
        fs::write(&dest, &rendered)
            .map_err(|err| format!("could not write to {}: {err}", dest.display()))?;
    }

    log!(Info, "Template generated: {}", template.display());
    Ok(())
}

fn execute_hook(cmd: &str) -> Result<()> {
    let mut cmd_iter = cmd.split_whitespace();
    let output = process::Command::new(
        cmd_iter
            .next()
            .ok_or("could not execute hook: No command provided".to_string())?,
    )
    .args(cmd_iter)
    .output()?;
    io::stdout().write_all(&output.stdout)?;
    io::stderr().write_all(&output.stderr)?;
    Ok(())
}
