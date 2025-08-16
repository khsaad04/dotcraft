mod cli;
mod colors;

use serde::Deserialize;
use std::{
    collections::HashMap,
    fs, io,
    os::unix::fs::symlink,
    path::{Component, Path, PathBuf},
    process::{exit, Command},
};

#[derive(Debug, Deserialize)]
struct Manifest {
    wallpaper: Option<PathBuf>,
    #[serde(default = "default_theme_option")]
    theme: String,
    #[serde(default = "default_variant_option")]
    variant: String,
    variables: Option<HashMap<String, String>>,
    files: HashMap<String, File>,
}

fn default_theme_option() -> String {
    "dark".to_string()
}

fn default_variant_option() -> String {
    "tonal_spot".to_string()
}

#[derive(Debug, Deserialize)]
struct File {
    target: Option<PathBuf>,
    dest: PathBuf,
    template: Option<PathBuf>,
    #[serde(default = "default_recursive_option")]
    recursive: bool,
    pre_hooks: Option<Vec<String>>,
    post_hooks: Option<Vec<String>>,
}

fn default_recursive_option() -> bool {
    false
}

impl TryFrom<&Path> for Manifest {
    type Error = Error;
    fn try_from(value: &Path) -> std::result::Result<Self, Self::Error> {
        let path = value
            .canonicalize()
            .map_err(|err| format!("invalid path {}: {err}", value.display()))?;
        let parent_dir = path
            .parent()
            .ok_or(format!("could not access parent dir of {}", path.display()))?;
        std::env::set_current_dir(parent_dir).map_err(|err| {
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

type ContextMap = HashMap<String, String>;

pub type Result<T> = std::result::Result<T, Error>;

pub struct Error {
    ctx: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.ctx)
    }
}

impl From<String> for Error {
    fn from(value: String) -> Self {
        Self { ctx: value }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self {
            ctx: value.to_string(),
        }
    }
}

impl From<flagge::Error> for Error {
    fn from(value: flagge::Error) -> Self {
        Self {
            ctx: value.to_string(),
        }
    }
}

enum LogLevel {
    Info,
    Warning,
    Error,
}

macro_rules! log {
    ($loglevel:ident, $($arg:tt)*) => {
        match LogLevel::$loglevel {
            LogLevel::Info => {
                print!("\x1b[0;32mINFO\x1b[0m: ");
                println!($($arg)*);
            }
            LogLevel::Warning => {
                print!("\x1b[0;33mWARNING\x1b[0m: ");
                println!($($arg)*);
            }
            LogLevel::Error => {
                eprint!("\x1b[0;31mERROR\x1b[0m: ");
                eprintln!($($arg)*);
            }
        }
    };
}

fn main() {
    if let Err(err) = entrypoint() {
        log!(Error, "{err}");
        exit(1);
    }
}

fn entrypoint() -> Result<()> {
    let args = cli::Cli::try_parse()?;

    let mut context: ContextMap = HashMap::new();
    let manifest = Manifest::try_from(args.manifest_path.as_path())?;

    let mut template_engine = upon::Engine::new();

    if let cli::SubCommand::Sync { force, ref name } = args.subcommand {
        if let Some(name) = name {
            if let Some(file) = manifest.files.get(name) {
                if let Some(pre_hook) = &file.pre_hooks {
                    for cmd in pre_hook.iter() {
                        log!(Info, "Executing pre-hook in {}: {}", name, cmd);
                        execute_hook(cmd)?;
                    }
                }

                if let Some(target) = &file.target {
                    symlink_dir_all(target, &file.dest, force, file.recursive).map_err(|err| {
                        format!("something went wrong while symlinking {name}:\n    {err}")
                    })?;
                }

                if let Some(template) = &file.template {
                    create_context_map(&mut context, &manifest)?;
                    generate_template(&file.dest, template, &context, &mut template_engine)
                        .map_err(|err| {
                            format!("something went wrong while generating {name}:\n    {err}")
                        })?;
                }

                if let Some(post_hook) = &file.post_hooks {
                    for cmd in post_hook.iter() {
                        log!(Info, "Executing post-hook in {}: {}", name, cmd);
                        execute_hook(cmd)?;
                    }
                }
            } else {
                return Err(format!("could not find {}", &name).into());
            }
        } else {
            if has_templates(&manifest) {
                create_context_map(&mut context, &manifest)?;
            }
            for (name, file) in manifest.files.iter() {
                if let Some(pre_hook) = &file.pre_hooks {
                    for cmd in pre_hook.iter() {
                        log!(Info, "Executing pre-hook in {}: {}", name, cmd);
                        execute_hook(cmd)?;
                    }
                }

                if let Some(target) = &file.target {
                    symlink_dir_all(target, &file.dest, force, file.recursive).map_err(|err| {
                        format!("something went wrong while symlinking {name}:\n    {err}")
                    })?;
                }

                if let Some(template) = &file.template {
                    generate_template(&file.dest, template, &context, &mut template_engine)
                        .map_err(|err| {
                            format!("something went wrong while generating {name}:\n    {err}")
                        })?;
                }

                if let Some(post_hook) = &file.post_hooks {
                    for cmd in post_hook.iter() {
                        log!(Info, "Executing post-hook in {}: {}", name, cmd);
                        execute_hook(cmd)?;
                    }
                }
            }
        }
    }

    if let cli::SubCommand::Link { force, ref name } = args.subcommand {
        if let Some(name) = name {
            if let Some(file) = manifest.files.get(name) {
                if let Some(target) = &file.target {
                    symlink_dir_all(target, &file.dest, force, file.recursive).map_err(|err| {
                        format!("something went wrong while symlinking {name}:\n    {err}")
                    })?;
                }
            } else {
                return Err(format!("could not find {}", &name).into());
            }
        } else {
            for (name, file) in manifest.files.iter() {
                if let Some(target) = &file.target {
                    symlink_dir_all(target, &file.dest, force, file.recursive).map_err(|err| {
                        format!("something went wrong while symlinking {name}:\n    {err}")
                    })?;
                }
            }
        }
    }

    if let cli::SubCommand::Generate { ref name } = args.subcommand {
        if let Some(name) = name {
            if let Some(file) = manifest.files.get(name) {
                if let Some(template) = &file.template {
                    create_context_map(&mut context, &manifest)?;
                    generate_template(&file.dest, template, &context, &mut template_engine)
                        .map_err(|err| {
                            format!("something went wrong while generating {name}:\n    {err}")
                        })?;
                }
            } else {
                return Err(format!("could not find {}", &name).into());
            }
        } else {
            if has_templates(&manifest) {
                create_context_map(&mut context, &manifest)?;
            }
            for (name, file) in manifest.files.iter() {
                if let Some(template) = &file.template {
                    generate_template(&file.dest, template, &context, &mut template_engine)
                        .map_err(|err| {
                            format!("something went wrong while generating {name}:\n    {err}")
                        })?;
                }
            }
        }
    }

    Ok(())
}

fn execute_hook(cmd: &str) -> Result<()> {
    let mut cmd_iter = cmd.split_whitespace();
    // TODO: using .spawn() inherits file descriptors (stdout, stderr, ...) from
    // the parent processs (dotcraft's process) which can mess up the order of
    // I/O between these hooks and dotcraft log messages. Find a possible fix
    // in the future. If there even is one that doesn't involve capturing the
    // stdout and stderr using .output() and writing them sequentially instead
    // of in the order they appeared.
    Command::new(
        cmd_iter
            .next()
            .ok_or("could not execute hook: No command provided".to_string())?,
    )
    .args(cmd_iter)
    .spawn()?;
    Ok(())
}

fn create_context_map(context: &mut ContextMap, manifest: &Manifest) -> Result<()> {
    if let Some(wallpaper) = &manifest.wallpaper {
        let wp_path = wallpaper
            .canonicalize()
            .map_err(|err| format!("could not find {}: {err}", wallpaper.display()))?;
        context.insert("wallpaper".to_string(), wp_path.display().to_string());
        colors::generate_material_colors(&wp_path, &manifest.theme, &manifest.variant, context)?;
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

fn has_templates(manifest: &Manifest) -> bool {
    for (_, file) in manifest.files.iter() {
        if file.template.is_some() {
            return true;
        }
    }
    false
}

fn resolve_home_dir(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    let home_dir =
        std::env::var("HOME").map_err(|err| format!("could not find home directory: {err}"))?;

    if let Some(prefix) = path.components().next() {
        if prefix == Component::Normal("~".as_ref()) {
            if let Ok(stripped_path) = path.strip_prefix("~") {
                let mut result = PathBuf::new();
                result.push(home_dir);
                result.push(stripped_path);
                return Ok(result);
            }
        }
    }
    Ok(path.to_path_buf())
}

fn symlink_dir_all(
    target: impl AsRef<Path>,
    dest: impl AsRef<Path>,
    force: bool,
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
            if !dest_parent_dir.exists() {
                fs::create_dir_all(dest_parent_dir).map_err(|err| {
                    format!("could not create dir {}: {err}", dest_parent_dir.display())
                })?;
                log!(Info, "Created dir: {}", dest_parent_dir.display());
            }
            symlink_dir_all(entry.path(), dest, force, recursive)?;
        }
    } else {
        symlink_file(&target, &dest, force)?;
    }
    Ok(())
}

fn symlink_file(target: impl AsRef<Path>, dest: impl AsRef<Path>, force: bool) -> Result<()> {
    let target = target.as_ref();
    let dest = dest.as_ref();

    match symlink(target, dest) {
        Ok(()) => {
            log!(Info, "Symlinked {} -> {}", target.display(), dest.display());
        }
        Err(err) => match err.kind() {
            io::ErrorKind::AlreadyExists => {
                if force {
                    log!(
                        Warning,
                        "Destination {} already exists. Removing",
                        dest.display()
                    );
                    std::fs::remove_file(dest).map_err(|err| {
                        format!("could not remove file {}: {err}", dest.display())
                    })?;
                    symlink(target, dest)?;
                    log!(Info, "Symlinked {} -> {}", target.display(), dest.display());
                } else if dest.is_symlink() {
                    if !dest.exists() {
                        log!(
                            Warning,
                            "Destination {} is a broken symlink. Ignoring",
                            dest.display()
                        );
                        std::fs::remove_file(dest).map_err(|err| {
                            format!("could not remove file {}: {err}", dest.display())
                        })?;
                        symlink(target, dest)?;
                        log!(Info, "Symlinked {} -> {}", target.display(), dest.display());
                    } else {
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
                    }
                } else {
                    log!(
                        Warning,
                        "Destination {} exists but it's not a symlink. Resolve manually",
                        dest.display()
                    );
                }
            }
            _ => {
                return Err(format!(
                    "could not symlink {} to {}: {err}",
                    target.display(),
                    dest.display()
                )
                .into());
            }
        },
    }
    Ok(())
}

fn generate_template(
    dest: impl AsRef<Path>,
    template: impl AsRef<Path>,
    context: &ContextMap,
    template_engine: &mut upon::Engine,
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

    if let Err(err) = fs::write(&dest, &rendered) {
        match err.kind() {
            io::ErrorKind::NotFound => {
                fs::create_dir_all(
                    dest.parent()
                        .ok_or(format!("could not access parent dir of {}", dest.display()))?,
                )?;
                fs::write(&dest, &rendered)
                    .map_err(|err| format!("could not write to {}: {err}", dest.display()))?;
            }
            _ => {
                return Err(format!("could not write to {}: {err}", dest.display()).into());
            }
        }
    }
    log!(Info, "Template generated: {}", template.display());
    Ok(())
}
