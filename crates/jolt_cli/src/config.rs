//! `jolt.toml` discovery and layering.
//!
//! A directory's settings are its parent's with the directory's own config
//! files layered on top. The walk up stops at a project root: a directory with
//! a VCS marker or a `root = true` config. Without one at or above the
//! invocation root, the invocation root is the project root.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

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

/// The `jolt.toml` file format, also used to print the effective config.
#[derive(Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(title = "Jolt configuration")]
pub(crate) struct ConfigFile {
    /// Marks this config as the project root when set to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) root: Option<bool>,
    /// Formatting options.
    #[serde(default)]
    pub(crate) format: FormatConfig,
    /// File selection options.
    #[serde(default)]
    pub(crate) files: Files,
}

#[derive(Default, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct Files {
    /// Source file globs to include, relative to this config's directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) include: Option<Vec<String>>,
    /// Source file globs to exclude, relative to this config's directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exclude: Option<Vec<String>>,
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
    fn new(base: &Path, format: FormatConfig, files: Files) -> Result<Self> {
        let globs = |patterns: Option<Vec<String>>| {
            patterns
                .map(|patterns| Globs::new(base, patterns).map(Arc::new))
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
    patterns: Vec<String>,
    set: GlobSet,
}

impl Globs {
    fn new(base: &Path, patterns: Vec<String>) -> Result<Self> {
        let mut set = GlobSetBuilder::new();
        for pattern in &patterns {
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .backslash_escape(true)
                .build()
                .map_err(|error| format!("invalid glob pattern `{pattern}`: {error}"))?;
            set.add(glob);
        }
        Ok(Self {
            base: base.to_path_buf(),
            patterns,
            set: set.build().map_err(|error| error.to_string())?,
        })
    }

    fn matches(&self, path: &Path) -> bool {
        path.strip_prefix(&self.base)
            .is_ok_and(|relative| self.set.is_match(relative))
    }
}

/// The effective settings for one directory.
#[derive(Clone)]
pub(crate) struct Settings {
    format: FormatConfig,
    include: Arc<Globs>,
    excludes: Vec<Arc<Globs>>,
    /// The config files that contributed, lowest precedence first.
    pub(crate) sources: Vec<PathBuf>,
}

impl Settings {
    fn defaults(root: &Path) -> Result<Self> {
        Ok(Self {
            format: FormatConfig::default(),
            include: Arc::new(Globs::new(root, vec![DEFAULT_INCLUDE.to_owned()])?),
            excludes: Vec::new(),
            sources: Vec::new(),
        })
    }

    fn apply(&mut self, layer: &Layer) {
        self.format = layer.format.or(self.format);
        if let Some(include) = &layer.include {
            self.include = Arc::clone(include);
        }
        self.excludes.extend(layer.exclude.iter().cloned());
    }

    fn add(&mut self, config: Config) {
        self.apply(&config.layer);
        self.sources.push(config.path);
    }

    pub(crate) fn options(&self) -> FormatOptions {
        self.format.resolve()
    }

    /// Whether the include and exclude globs select `path`.
    pub(crate) fn selects(&self, path: &Path) -> bool {
        self.include.matches(path) && !self.excludes.iter().any(|globs| globs.matches(path))
    }

    /// The effective config as a `jolt.toml`.
    pub(crate) fn to_file(&self) -> ConfigFile {
        let exclude: Vec<String> = self
            .excludes
            .iter()
            .flat_map(|globs| globs.patterns.iter().cloned())
            .collect();
        ConfigFile {
            root: None,
            format: self.options().into(),
            files: Files {
                include: Some(self.include.patterns.clone()),
                exclude: (!exclude.is_empty()).then_some(exclude),
            },
        }
    }
}

/// Where settings come from, as chosen on the command line.
pub(crate) struct Sources<'a> {
    pub(crate) cwd: &'a Path,
    pub(crate) format: FormatConfig,
    pub(crate) include: &'a [String],
    pub(crate) exclude: &'a [String],
    /// Load only this config file.
    pub(crate) config: Option<&'a Path>,
    /// Load no config files.
    pub(crate) no_config: bool,
}

/// Resolves settings for directories under one invocation root.
pub(crate) struct Resolver {
    root: PathBuf,
    /// Whether a project root exists at or above `root`.
    bounded: bool,
    /// Settings for every directory when discovery is off.
    fixed: Option<Settings>,
    cli: Layer,
    discovered: HashMap<PathBuf, Settings>,
}

impl Resolver {
    pub(crate) fn new(sources: &Sources<'_>, root: &Path) -> Result<Self> {
        let cli = Layer::new(
            sources.cwd,
            sources.format,
            Files {
                include: (!sources.include.is_empty()).then(|| sources.include.to_vec()),
                exclude: (!sources.exclude.is_empty()).then(|| sources.exclude.to_vec()),
            },
        )?;
        let fixed = if let Some(path) = sources.config {
            let path = absolute(sources.cwd, path);
            if !path.is_file() {
                return Err(format!("{}: config file does not exist", path.display()));
            }
            let base = path.parent().unwrap_or(&path).to_path_buf();
            let mut settings = Settings::defaults(root)?;
            settings.add(load(path, &base)?);
            Some(settings)
        } else if sources.no_config {
            Some(Settings::defaults(root)?)
        } else {
            None
        };
        let mut bounded = false;
        if fixed.is_none() {
            for dir in root.ancestors() {
                if is_project_root(dir, &configs_in(dir)?) {
                    bounded = true;
                    break;
                }
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            bounded,
            fixed,
            cli,
            discovered: HashMap::new(),
        })
    }

    /// The settings for files in `dir`, command-line overrides included.
    pub(crate) fn settings(&mut self, dir: &Path) -> Result<Settings> {
        let mut settings = match &self.fixed {
            Some(settings) => settings.clone(),
            None => self.discover(dir)?,
        };
        settings.apply(&self.cli);
        Ok(settings)
    }

    fn discover(&mut self, dir: &Path) -> Result<Settings> {
        if let Some(settings) = self.discovered.get(dir) {
            return Ok(settings.clone());
        }
        let configs = configs_in(dir)?;
        let is_root = is_project_root(dir, &configs) || (!self.bounded && dir == self.root);
        let mut settings = match dir.parent() {
            Some(parent) if !is_root => self.discover(parent)?,
            _ => Settings::defaults(&self.root)?,
        };
        for config in configs {
            settings.add(config);
        }
        self.discovered.insert(dir.to_path_buf(), settings.clone());
        Ok(settings)
    }
}

/// A parsed config file.
struct Config {
    path: PathBuf,
    root: bool,
    layer: Layer,
}

/// The config files in `dir`, lowest precedence first.
fn configs_in(dir: &Path) -> Result<Vec<Config>> {
    CONFIG_FILES
        .iter()
        .map(|name| dir.join(name))
        .filter(|path| path.is_file())
        .map(|path| load(path, dir))
        .collect()
}

fn is_project_root(dir: &Path, configs: &[Config]) -> bool {
    configs.iter().any(|config| config.root)
        || VCS_MARKERS.iter().any(|marker| dir.join(marker).exists())
}

/// Parses the config file at `path`, whose globs are relative to `base`.
fn load(path: PathBuf, base: &Path) -> Result<Config> {
    let prefix = |error: String| format!("{}: {}", path.display(), error.trim_end());
    let text = fs::read_to_string(&path).map_err(|error| prefix(error.to_string()))?;
    let file: ConfigFile = toml::from_str(&text).map_err(|error| prefix(error.to_string()))?;
    let layer = Layer::new(base, file.format, file.files).map_err(prefix)?;
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
