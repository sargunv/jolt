//! `jolt.toml` discovery and layering.
//!
//! A directory's settings are its parent's with the directory's own config
//! files layered on top. The walk up stops at a project root: a directory with
//! a VCS marker or a `root = true` config. Without one above a target, it stops
//! at the invocation directory for a target under it, or else at the target.

use std::{
    collections::HashMap,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Args;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use jolt_config::FormatConfig;
use jolt_formatter::FormatOptions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Result;

/// Config file locations in a directory, lowest precedence first.
pub(crate) const CONFIG_FILES: [&str; 3] =
    [".config/jolt/config.toml", ".config/jolt.toml", "jolt.toml"];
const VCS_MARKERS: [&str; 4] = [".git", ".hg", ".jj", ".svn"];
const DEFAULT_INCLUDE: &str = "**/*.java";

/// File selection and config loading, as chosen on the command line.
#[derive(Debug, Default, Args)]
pub(crate) struct ConfigArgs {
    /// Include source files matching this glob.
    #[arg(long)]
    include: Vec<String>,

    /// Exclude source files matching this glob.
    #[arg(long)]
    exclude: Vec<String>,

    /// Load only this config file.
    #[arg(long, conflicts_with = "no_config")]
    config: Option<PathBuf>,

    /// Disable discovered project configs.
    #[arg(long)]
    no_config: bool,
}

/// The `jolt.toml` file format.
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(title = "Jolt configuration")]
pub(crate) struct ConfigFile {
    /// Marks this config as the project root when set to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<bool>,
    /// Formatting options.
    #[serde(default)]
    format: FormatConfig,
    /// File selection options.
    #[serde(default)]
    files: Files,
}

#[derive(Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Files {
    /// Source file globs to include, relative to this config's directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    include: Option<Vec<String>>,
    /// Source file globs to exclude, relative to this config's directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude: Option<Vec<String>>,
}

impl ConfigFile {
    /// The config `jolt config init` writes: every option at its default.
    pub(crate) fn initial() -> Self {
        Self {
            root: Some(true),
            format: FormatOptions::default().into(),
            files: Files {
                include: Some(vec![DEFAULT_INCLUDE.to_owned()]),
                exclude: None,
            },
        }
    }
}

/// One parsed config file, or the command-line overrides.
struct Layer {
    format: FormatConfig,
    include: Option<Arc<Globs>>,
    exclude: Option<Arc<Globs>>,
}

impl Layer {
    /// A layer whose globs are relative to `base`, from the config file
    /// `origin` (`None` for the command line).
    fn new(base: &Path, origin: Option<&Path>, format: FormatConfig, files: Files) -> Result<Self> {
        let globs = |patterns: Option<Vec<String>>| {
            patterns
                .map(|patterns| Globs::new(base, origin, patterns).map(Arc::new))
                .transpose()
        };
        Ok(Self {
            format,
            include: globs(files.include)?,
            exclude: globs(files.exclude)?,
        })
    }
}

/// Patterns matched against paths relative to `base`.
struct Globs {
    base: PathBuf,
    /// The config file that sets them, or `None` for the command line.
    origin: Option<PathBuf>,
    patterns: Vec<String>,
    set: GlobSet,
}

impl Globs {
    fn new(base: &Path, origin: Option<&Path>, patterns: Vec<String>) -> Result<Self> {
        let mut set = GlobSetBuilder::new();
        for pattern in &patterns {
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .backslash_escape(true)
                .build()
                .map_err(|error| format!("invalid glob pattern `{pattern}`: {error}"))?;
            set.add(glob);
        }
        let set = set
            .build()
            .map_err(|error| format!("invalid glob patterns: {error}"))?;
        Ok(Self {
            base: base.to_path_buf(),
            origin: origin.map(Path::to_path_buf),
            patterns,
            set,
        })
    }

    fn matches(&self, path: &Path) -> bool {
        path.strip_prefix(&self.base)
            .is_ok_and(|relative| self.set.is_match(relative))
    }

    /// Where the globs come from, for `jolt config resolve`.
    fn origin(&self) -> String {
        self.origin.as_ref().map_or_else(
            || "command line".to_owned(),
            |path| path.display().to_string(),
        )
    }
}

/// The effective settings for one directory.
#[derive(Clone, Default)]
pub(crate) struct Settings {
    format: FormatConfig,
    /// `None` selects every source file.
    include: Option<Arc<Globs>>,
    excludes: Vec<Arc<Globs>>,
    /// The config files that contributed, lowest precedence first.
    pub(crate) sources: Vec<PathBuf>,
}

impl Settings {
    fn apply(&mut self, layer: &Layer) {
        self.format = layer.format.or(self.format);
        if let Some(include) = &layer.include {
            self.include = Some(Arc::clone(include));
        }
        self.excludes.extend(layer.exclude.iter().cloned());
    }

    fn add(&mut self, config: &Config) {
        self.apply(&config.layer);
        self.sources.push(config.path.clone());
    }

    pub(crate) fn options(&self) -> FormatOptions {
        self.format.resolve()
    }

    /// Whether the include and exclude globs select `path`, a file found in
    /// a directory.
    pub(crate) fn selects(&self, path: &Path) -> bool {
        self.include
            .as_ref()
            .is_none_or(|globs| globs.matches(path))
            && !self.excludes(path)
    }

    /// Whether the exclude globs match `path`. They apply to files named on
    /// the command line too; the include globs don't.
    pub(crate) fn excludes(&self, path: &Path) -> bool {
        self.excludes.iter().any(|globs| globs.matches(path))
    }

    /// The effective config as a `jolt.toml`, each glob annotated with the
    /// config file it is relative to.
    pub(crate) fn render(&self) -> Result<String> {
        #[derive(Serialize)]
        struct Format {
            format: FormatConfig,
        }
        let quote = |pattern: &str| toml::Value::String(pattern.to_owned()).to_string();
        let mut text = toml::to_string(&Format {
            format: self.options().into(),
        })
        .map_err(|error| format!("failed to serialize config: {error}"))?;
        text.push_str(
            "\n[files]\n# Globs are relative to the directory of the config that sets them.\n",
        );
        match &self.include {
            Some(globs) => {
                let patterns: Vec<String> = globs.patterns.iter().map(|p| quote(p)).collect();
                let _ = writeln!(
                    text,
                    "include = [{}] # {}",
                    patterns.join(", "),
                    globs.origin()
                );
            }
            None => {
                let _ = writeln!(text, "include = [{}] # default", quote(DEFAULT_INCLUDE));
            }
        }
        if !self.excludes.is_empty() {
            text.push_str("exclude = [\n");
            for globs in &self.excludes {
                for pattern in &globs.patterns {
                    let _ = writeln!(text, "  {}, # {}", quote(pattern), globs.origin());
                }
            }
            text.push_str("]\n");
        }
        Ok(text)
    }
}

/// Resolves the settings for the directories of one invocation, parsing each
/// config file once and resolving each directory once.
pub(crate) struct Resolver {
    cwd: PathBuf,
    cli: Layer,
    /// The settings for every directory when discovery is off.
    fixed: Option<Arc<Settings>>,
    /// The parsed config files of each directory seen so far.
    dirs: HashMap<PathBuf, Dir>,
    /// The settings by the directory discovery falls back to when there is no
    /// project root, then by directory.
    resolved: HashMap<PathBuf, HashMap<PathBuf, Arc<Settings>>>,
}

impl Resolver {
    pub(crate) fn new(cwd: &Path, args: &ConfigArgs, format: FormatConfig) -> Result<Self> {
        let cli = Layer::new(
            cwd,
            None,
            format,
            Files {
                include: (!args.include.is_empty()).then(|| args.include.clone()),
                exclude: (!args.exclude.is_empty()).then(|| args.exclude.clone()),
            },
        )?;
        let fixed = if let Some(path) = &args.config {
            let path = absolute(cwd, path);
            if !path.is_file() {
                return Err(format!("{}: config file does not exist", path.display()));
            }
            let base = path.parent().unwrap_or(&path).to_path_buf();
            let mut settings = Settings::default();
            settings.add(&load(path, &base)?);
            Some(settings)
        } else if args.no_config {
            Some(Settings::default())
        } else {
            None
        };
        Ok(Self {
            cwd: cwd.to_path_buf(),
            fixed: fixed.map(|mut settings| {
                settings.apply(&cli);
                Arc::new(settings)
            }),
            cli,
            dirs: HashMap::new(),
            resolved: HashMap::new(),
        })
    }

    /// The settings for files in `dir`, command-line overrides included.
    /// `dir` is at or under `target`, the path named on the command line.
    pub(crate) fn settings(&mut self, dir: &Path, target: &Path) -> Result<Arc<Settings>> {
        if let Some(settings) = &self.fixed {
            return Ok(Arc::clone(settings));
        }
        // So `jolt fmt .`, `jolt fmt a/A.java` and stdin named `a/A.java` all
        // see the same config files.
        let fallback = if target.starts_with(&self.cwd) {
            &self.cwd
        } else {
            target
        };
        if let Some(settings) = self.resolved.get(fallback).and_then(|dirs| dirs.get(dir)) {
            return Ok(Arc::clone(settings));
        }
        let fallback = fallback.to_path_buf();
        let mut settings = Settings::default();
        for ancestor in self.discovery_path(dir, &fallback)? {
            for config in &self.dirs[ancestor].configs {
                settings.add(config);
            }
        }
        settings.apply(&self.cli);
        let settings = Arc::new(settings);
        self.resolved
            .entry(fallback)
            .or_default()
            .insert(dir.to_path_buf(), Arc::clone(&settings));
        Ok(settings)
    }

    /// The directories whose configs apply to `dir`, outermost first: up to
    /// the nearest project root, or else up to `fallback`.
    fn discovery_path<'p>(&mut self, dir: &'p Path, fallback: &Path) -> Result<Vec<&'p Path>> {
        let mut path = Vec::new();
        let mut bounded = false;
        for ancestor in dir.ancestors() {
            path.push(ancestor);
            if !self.dirs.contains_key(ancestor) {
                self.dirs
                    .insert(ancestor.to_path_buf(), Dir::load(ancestor)?);
            }
            if self.dirs[ancestor].project_root {
                bounded = true;
                break;
            }
        }
        if !bounded && let Some(end) = path.iter().position(|ancestor| *ancestor == fallback) {
            path.truncate(end + 1);
        }
        path.reverse();
        Ok(path)
    }
}

/// The config files in a directory, lowest precedence first, and whether the
/// directory is a project root.
struct Dir {
    configs: Vec<Config>,
    project_root: bool,
}

impl Dir {
    fn load(dir: &Path) -> Result<Self> {
        let configs = CONFIG_FILES
            .iter()
            .map(|name| dir.join(name))
            .filter(|path| path.is_file())
            .map(|path| load(path, dir))
            .collect::<Result<Vec<_>>>()?;
        let project_root = configs.iter().any(|config| config.root)
            || VCS_MARKERS.iter().any(|marker| dir.join(marker).exists());
        Ok(Self {
            configs,
            project_root,
        })
    }
}

/// A parsed config file.
struct Config {
    path: PathBuf,
    root: bool,
    layer: Layer,
}

/// Parses the config file at `path`, whose globs are relative to `base`.
fn load(path: PathBuf, base: &Path) -> Result<Config> {
    let prefix = |error: String| format!("{}: {}", path.display(), error.trim_end());
    let text = fs::read_to_string(&path).map_err(|error| prefix(error.to_string()))?;
    let file: ConfigFile = toml::from_str(&text).map_err(|error| prefix(error.to_string()))?;
    let layer = Layer::new(base, Some(&path), file.format, file.files).map_err(prefix)?;
    Ok(Config {
        root: file.root == Some(true),
        layer,
        path,
    })
}

/// `path` relative to `cwd`, without `.` components.
pub(crate) fn absolute(cwd: &Path, path: &Path) -> PathBuf {
    cwd.join(path).components().collect()
}
