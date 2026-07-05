use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use figment::{
    Figment,
    providers::{Format, Toml},
};
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use jolt_formatter::FormatOptions;
use serde::{Deserialize, Serialize};

use crate::error::CliError;

use super::CliFormatOptions;

const DEFAULT_INCLUDE: &[&str] = &["**/*.java"];
const VCS_MARKERS: &[&str] = &[".git", ".hg", ".jj", ".svn"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DefaultConfig {
    pub(crate) options: FormatOptions,
    pub(crate) include: Vec<String>,
}

pub(crate) fn default_config() -> DefaultConfig {
    DefaultConfig {
        options: FormatOptions::default(),
        include: default_include_patterns(),
    }
}

pub(crate) fn default_file_config() -> FileConfig {
    FileConfig::from_default_config(default_config())
}

trait WithConfigSource {
    fn with_source(self, source: &Path) -> Self;
}

impl WithConfigSource for CliError {
    fn with_source(self, source: &Path) -> Self {
        self.with_prefix(source.display())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PatternList {
    base_dir: PathBuf,
    globset: GlobSet,
}

impl PatternList {
    pub(crate) fn new(base_dir: impl Into<PathBuf>, patterns: &[String]) -> Result<Self, CliError> {
        let base_dir = base_dir.into();
        let mut builder = GlobSetBuilder::new();

        for pattern in patterns {
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .backslash_escape(true)
                .build()
                .map_err(|error| {
                    CliError::new(format!("invalid glob pattern `{pattern}`: {error}"))
                })?;
            builder.add(glob);
        }

        let globset = builder
            .build()
            .map_err(|error| CliError::new(format!("invalid glob set: {error}")))?;

        Ok(Self { base_dir, globset })
    }

    pub(crate) fn matches(&self, path: &Path) -> bool {
        let Ok(relative) = path.strip_prefix(&self.base_dir) else {
            return false;
        };
        self.globset.is_match(relative)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedConfig {
    pub(crate) options: FormatOptions,
    pub(crate) include: PatternList,
    pub(crate) excludes: Vec<PatternList>,
}

#[derive(Clone, Debug, Default)]
struct SparseConfig {
    line_width: Option<u16>,
    indent_width: Option<u8>,
    use_tabs: Option<bool>,
    include: Option<PatternList>,
    exclude: Vec<PatternList>,
}

#[derive(Clone, Debug)]
pub(crate) struct ConfigResolver {
    invocation_root: PathBuf,
    cli_options: CliFormatOptions,
    cli_include: Option<PatternList>,
    cli_exclude: Option<PatternList>,
    explicit_config: Option<SparseConfig>,
    no_config: bool,
    resolved_by_dir: HashMap<PathBuf, ResolvedConfig>,
}

impl ConfigResolver {
    pub(crate) fn new(
        cwd: &Path,
        invocation_root: PathBuf,
        cli_options: CliFormatOptions,
        cli_include_patterns: &[String],
        cli_exclude_patterns: &[String],
        explicit_config: Option<&Path>,
        no_config: bool,
    ) -> Result<Self, CliError> {
        validate_options(cli_options, "CLI")?;

        let cli_include = (!cli_include_patterns.is_empty())
            .then(|| PatternList::new(cwd, cli_include_patterns))
            .transpose()?;
        let cli_exclude = (!cli_exclude_patterns.is_empty())
            .then(|| PatternList::new(cwd, cli_exclude_patterns))
            .transpose()?;
        let explicit_config = explicit_config
            .map(|path| load_explicit_config(cwd, path))
            .transpose()?;
        Ok(Self {
            invocation_root,
            cli_options,
            cli_include,
            cli_exclude,
            explicit_config,
            no_config,
            resolved_by_dir: HashMap::new(),
        })
    }

    pub(crate) fn resolve_for_dir(&mut self, dir: &Path) -> Result<ResolvedConfig, CliError> {
        if let Some(config) = self.resolved_by_dir.get(dir) {
            return Ok(config.clone());
        }

        let mut builder = ConfigBuilder::new(&self.invocation_root)?;

        if self.explicit_config.is_none() && !self.no_config {
            let project_root = find_project_root(dir, &self.invocation_root)?;
            for config in Self::discovered_configs(&project_root, dir)? {
                builder.apply_sparse(&config);
            }
        }

        if let Some(config) = &self.explicit_config {
            builder.apply_sparse(config);
        }

        builder.apply_cli_options(self.cli_options);

        if let Some(include) = &self.cli_include {
            builder.include = include.clone();
        }
        if let Some(exclude) = &self.cli_exclude {
            builder.excludes.push(exclude.clone());
        }

        let resolved = builder.finish();
        self.resolved_by_dir
            .insert(dir.to_path_buf(), resolved.clone());
        Ok(resolved)
    }

    fn discovered_configs(project_root: &Path, dir: &Path) -> Result<Vec<SparseConfig>, CliError> {
        let mut configs = Vec::new();

        for ancestor in ancestors_from_root_to_dir(project_root, dir) {
            for config_path in config_paths_for_dir(&ancestor) {
                if config_path.path.is_file() {
                    configs.push(load_config_at(&config_path.path, &config_path.base_dir)?);
                }
            }
        }

        Ok(configs)
    }
}

#[derive(Clone, Debug)]
struct ConfigBuilder {
    options: FormatOptions,
    include: PatternList,
    excludes: Vec<PatternList>,
}

impl ConfigBuilder {
    fn new(invocation_root: &Path) -> Result<Self, CliError> {
        let default = default_config();
        Ok(Self {
            options: default.options,
            include: PatternList::new(invocation_root, &default.include)?,
            excludes: Vec::new(),
        })
    }

    fn apply_sparse(&mut self, sparse: &SparseConfig) {
        if let Some(line_width) = sparse.line_width {
            self.options.line_width = line_width;
        }
        if let Some(indent_width) = sparse.indent_width {
            self.options.indent_width = indent_width;
        }
        if let Some(use_tabs) = sparse.use_tabs {
            self.options.use_tabs = use_tabs;
        }
        if let Some(include) = &sparse.include {
            self.include = include.clone();
        }
        self.excludes.extend(sparse.exclude.clone());
    }

    fn apply_cli_options(&mut self, options: CliFormatOptions) {
        if let Some(line_width) = options.line_width {
            self.options.line_width = line_width;
        }
        if let Some(indent_width) = options.indent_width {
            self.options.indent_width = indent_width;
        }
        if let Some(use_tabs) = options.use_tabs {
            self.options.use_tabs = use_tabs;
        }
    }

    fn finish(self) -> ResolvedConfig {
        ResolvedConfig {
            options: self.options,
            include: self.include,
            excludes: self.excludes,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct FileConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<FileFormatConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<FileSelectionConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ProjectRootConfig {
    root: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct FileFormatConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    line_width: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    indent_width: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_tabs: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct FileSelectionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    include: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exclude: Option<Vec<String>>,
}

fn load_explicit_config(cwd: &Path, path: &Path) -> Result<SparseConfig, CliError> {
    let path = absolutize(cwd, path);
    if !path.is_file() {
        return Err(CliError::new(format!(
            "{}: config file does not exist",
            path.display()
        )));
    }

    let base_dir = path.parent().map_or_else(PathBuf::new, Path::to_path_buf);
    load_config_at(&path, &base_dir)
}

fn load_config_at(path: &Path, base_dir: &Path) -> Result<SparseConfig, CliError> {
    let file_config = Figment::from(Toml::file(path))
        .extract::<FileConfig>()
        .map_err(|error| CliError::new(format!("{}: {error}", path.display())))?;
    file_config.into_sparse(path, base_dir)
}

impl FileConfig {
    fn from_default_config(default: DefaultConfig) -> Self {
        Self {
            root: Some(true),
            format: Some(FileFormatConfig {
                line_width: Some(default.options.line_width),
                indent_width: Some(default.options.indent_width),
                use_tabs: Some(default.options.use_tabs),
            }),
            files: Some(FileSelectionConfig {
                include: Some(default.include),
                exclude: None,
            }),
        }
    }

    fn into_sparse(self, path: &Path, base_dir: &Path) -> Result<SparseConfig, CliError> {
        let FileConfig { format, files, .. } = self;

        let sparse_options = CliFormatOptions {
            line_width: format.as_ref().and_then(|format| format.line_width),
            indent_width: format.as_ref().and_then(|format| format.indent_width),
            use_tabs: format.as_ref().and_then(|format| format.use_tabs),
        };
        validate_options(sparse_options, &path.display().to_string())?;

        let include = files
            .as_ref()
            .and_then(|files| files.include.as_ref())
            .cloned()
            .map(|patterns| PatternList::new(base_dir, &patterns))
            .transpose()
            .map_err(|error| error.with_source(path))?;
        let exclude = files
            .as_ref()
            .and_then(|files| files.exclude.as_ref())
            .cloned()
            .map(|patterns| PatternList::new(base_dir, &patterns).map(|list| vec![list]))
            .transpose()
            .map_err(|error| error.with_source(path))?
            .unwrap_or_default();

        Ok(SparseConfig {
            line_width: sparse_options.line_width,
            indent_width: sparse_options.indent_width,
            use_tabs: sparse_options.use_tabs,
            include,
            exclude,
        })
    }
}

fn validate_options(options: CliFormatOptions, source: &str) -> Result<(), CliError> {
    if options.line_width == Some(0) {
        return Err(CliError::new(format!(
            "{source}: line-width must be greater than zero"
        )));
    }
    if options.indent_width == Some(0) {
        return Err(CliError::new(format!(
            "{source}: indent-width must be greater than zero"
        )));
    }

    Ok(())
}

fn default_include_patterns() -> Vec<String> {
    DEFAULT_INCLUDE.iter().map(ToString::to_string).collect()
}

#[derive(Clone, Debug)]
struct ConfigPath {
    path: PathBuf,
    base_dir: PathBuf,
}

fn config_paths_for_dir(dir: &Path) -> Vec<ConfigPath> {
    let base_dir = dir.to_path_buf();
    vec![
        ConfigPath {
            path: dir.join(".config/jolt/config.toml"),
            base_dir: base_dir.clone(),
        },
        ConfigPath {
            path: dir.join(".config/jolt.toml"),
            base_dir: base_dir.clone(),
        },
        ConfigPath {
            path: dir.join("jolt.toml"),
            base_dir,
        },
    ]
}

fn find_project_root(dir: &Path, fallback: &Path) -> Result<PathBuf, CliError> {
    for ancestor in dir.ancestors() {
        if has_vcs_marker(ancestor) || has_root_config(ancestor)? {
            return Ok(ancestor.to_path_buf());
        }
    }

    Ok(fallback.to_path_buf())
}

fn has_vcs_marker(dir: &Path) -> bool {
    VCS_MARKERS.iter().any(|marker| dir.join(marker).exists())
}

fn has_root_config(dir: &Path) -> Result<bool, CliError> {
    for config_path in config_paths_for_dir(dir) {
        if !config_path.path.is_file() {
            continue;
        }

        let config = Figment::from(Toml::file(&config_path.path))
            .extract::<ProjectRootConfig>()
            .map_err(|error| CliError::new(format!("{}: {error}", config_path.path.display())))?;
        if config.root == Some(true) {
            return Ok(true);
        }
    }

    Ok(false)
}

fn ancestors_from_root_to_dir(root: &Path, dir: &Path) -> Vec<PathBuf> {
    let mut ancestors = dir
        .ancestors()
        .filter(|ancestor| ancestor.starts_with(root))
        .map(Path::to_path_buf)
        .collect::<Vec<_>>();
    ancestors.reverse();
    ancestors
}

pub(crate) fn absolutize(cwd: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    }
}
