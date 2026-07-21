use std::{
    collections::{BTreeSet, HashMap},
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use jolt_formatter::FormatOptions;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use toml_edit::DocumentMut;

use crate::error::CliError;

const DEFAULT_INCLUDE: &[&str] = &["**/*.java", "**/*.kt", "**/*.kts"];
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
    patterns: Vec<String>,
    globset: GlobSet,
}

impl PatternList {
    pub(crate) fn new(base_dir: impl Into<PathBuf>, patterns: &[String]) -> Result<Self, CliError> {
        let base_dir = base_dir.into();
        let patterns = patterns.to_vec();
        let mut builder = GlobSetBuilder::new();

        for pattern in &patterns {
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

        Ok(Self {
            base_dir,
            patterns,
            globset,
        })
    }

    pub(crate) fn matches(&self, path: &Path) -> bool {
        let Ok(relative) = path.strip_prefix(&self.base_dir) else {
            return false;
        };
        self.globset.is_match(relative)
    }

    pub(crate) fn patterns(&self) -> &[String] {
        &self.patterns
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResolvedConfig {
    pub(crate) options: FormatOptions,
    pub(crate) include: PatternList,
    pub(crate) excludes: Vec<PatternList>,
    sources: ResolvedConfigSources,
}

impl ResolvedConfig {
    pub(crate) fn matches_path(&self, path: &Path) -> bool {
        self.include.matches(path) && !self.excludes.iter().any(|exclude| exclude.matches(path))
    }
}

/// Sparse formatter options shared by `jolt.toml`'s `[format]` table, CLI
/// overrides, and the rendered effective configuration.
///
/// All fields are optional so a value can be left unset, letting it fall
/// through to a less specific config layer, a CLI flag, or the built-in
/// default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct FormatOptionsPatch {
    /// Preferred maximum rendered line width.
    #[schemars(range(min = 1, max = 65535))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) line_width: Option<u16>,
    /// Number of spaces per indentation level when `use-tabs` is false.
    #[schemars(range(min = 1, max = 255))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) indent_width: Option<u8>,
    /// Whether indentation should use tabs instead of spaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) use_tabs: Option<bool>,
}

/// Sparse file-selection overlay shared by `jolt.toml`'s `[files]` table, CLI
/// overrides, and the rendered effective configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct FileSelectionPatch {
    /// Source file globs to include.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) include: Option<Vec<String>>,
    /// Source file globs to exclude.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exclude: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
struct SparseConfig {
    source: ValueSource,
    format: FormatOptionsPatch,
    include: Option<PatternList>,
    exclude: Option<PatternList>,
}

#[derive(Clone, Debug)]
struct SourceValue<T> {
    value: T,
    source: ValueSource,
}

impl<T> SourceValue<T> {
    fn new(value: T, source: ValueSource) -> Self {
        Self { value, source }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum ValueSource {
    Default,
    Config(PathBuf),
    Cli,
}

impl ValueSource {
    fn label(&self) -> String {
        match self {
            Self::Default => "default".to_owned(),
            Self::Config(path) => path.display().to_string(),
            Self::Cli => "CLI".to_owned(),
        }
    }
}

/// Provenance for each field of a resolved [`FormatOptions`], tracking which
/// config layer, CLI flag, or built-in default contributed the final value.
#[derive(Clone, Debug)]
struct FormatOptionsSources {
    line_width: ValueSource,
    indent_width: ValueSource,
    use_tabs: ValueSource,
}

/// A [`FormatOptions`] under construction, tracking the source of each field
/// as [`FormatOptionsPatch`] overlays are applied in priority order.
#[derive(Clone, Debug)]
struct SourcedFormatOptions {
    line_width: SourceValue<u16>,
    indent_width: SourceValue<u8>,
    use_tabs: SourceValue<bool>,
}

impl SourcedFormatOptions {
    fn new(options: FormatOptions, source: ValueSource) -> Self {
        Self {
            line_width: SourceValue::new(options.line_width, source.clone()),
            indent_width: SourceValue::new(options.indent_width, source.clone()),
            use_tabs: SourceValue::new(options.use_tabs, source),
        }
    }

    fn apply_patch(&mut self, patch: FormatOptionsPatch, source: &ValueSource) {
        if let Some(line_width) = patch.line_width {
            self.line_width = SourceValue::new(line_width, source.clone());
        }
        if let Some(indent_width) = patch.indent_width {
            self.indent_width = SourceValue::new(indent_width, source.clone());
        }
        if let Some(use_tabs) = patch.use_tabs {
            self.use_tabs = SourceValue::new(use_tabs, source.clone());
        }
    }

    fn into_parts(self) -> (FormatOptions, FormatOptionsSources) {
        (
            FormatOptions {
                line_width: self.line_width.value,
                indent_width: self.indent_width.value,
                use_tabs: self.use_tabs.value,
            },
            FormatOptionsSources {
                line_width: self.line_width.source,
                indent_width: self.indent_width.source,
                use_tabs: self.use_tabs.source,
            },
        )
    }
}

#[derive(Clone, Debug)]
struct ResolvedConfigSources {
    format: FormatOptionsSources,
    include: ValueSource,
    excludes: Vec<ValueSource>,
}

#[derive(Clone, Debug)]
pub(crate) struct ConfigGraph {
    invocation_root: PathBuf,
    cli_options: FormatOptionsPatch,
    cli_include: Option<PatternList>,
    cli_exclude: Option<PatternList>,
    explicit_config: Option<SparseConfig>,
    no_config: bool,
    defaults: ConfigDefaults,
    config_paths_by_dir: HashMap<PathBuf, Vec<ConfigPath>>,
    boundary_by_dir: HashMap<PathBuf, bool>,
    project_root_by_dir: HashMap<PathBuf, PathBuf>,
    directory_config_by_dir: HashMap<PathBuf, DirectoryConfig>,
    discovered_by_dir: HashMap<PathBuf, ConfigBuilder>,
    resolved_by_dir: HashMap<PathBuf, ResolvedConfig>,
}

impl ConfigGraph {
    pub(crate) fn new(
        cwd: &Path,
        invocation_root: PathBuf,
        cli_options: FormatOptionsPatch,
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
        let defaults = ConfigDefaults::new(&invocation_root)?;
        Ok(Self {
            invocation_root,
            cli_options,
            cli_include,
            cli_exclude,
            explicit_config,
            no_config,
            defaults,
            config_paths_by_dir: HashMap::new(),
            boundary_by_dir: HashMap::new(),
            project_root_by_dir: HashMap::new(),
            directory_config_by_dir: HashMap::new(),
            discovered_by_dir: HashMap::new(),
            resolved_by_dir: HashMap::new(),
        })
    }

    pub(crate) fn resolve_for_dir(&mut self, dir: &Path) -> Result<ResolvedConfig, CliError> {
        if let Some(config) = self.resolved_by_dir.get(dir) {
            return Ok(config.clone());
        }

        let mut builder = if self.explicit_config.is_some() || self.no_config {
            ConfigBuilder::new(&self.defaults)
        } else {
            self.discovered_builder_for_dir(dir)?
        };

        if let Some(config) = &self.explicit_config {
            builder.apply_sparse(config);
        }

        builder.apply_cli_options(self.cli_options);

        if let Some(include) = &self.cli_include {
            builder.include = SourceValue::new(include.clone(), ValueSource::Cli);
        }
        if let Some(exclude) = &self.cli_exclude {
            builder
                .excludes
                .push(SourceValue::new(exclude.clone(), ValueSource::Cli));
        }

        let resolved = builder.finish();
        self.resolved_by_dir
            .insert(dir.to_path_buf(), resolved.clone());
        Ok(resolved)
    }

    pub(crate) fn discovered_config_paths_for_dir(
        &mut self,
        dir: &Path,
    ) -> Result<Vec<PathBuf>, CliError> {
        let project_root = self.project_root_for_dir(dir)?;
        let mut paths = Vec::new();
        for ancestor in ancestors_from_root_to_dir(&project_root, dir) {
            paths.extend(
                self.config_paths_for_dir(&ancestor)
                    .into_iter()
                    .map(|config_path| config_path.path),
            );
        }
        Ok(paths)
    }

    fn discovered_builder_for_dir(&mut self, dir: &Path) -> Result<ConfigBuilder, CliError> {
        if let Some(builder) = self.discovered_by_dir.get(dir) {
            return Ok(builder.clone());
        }

        let project_root = self.project_root_for_dir(dir)?;
        self.discovered_builder_from_root(&project_root, dir)
    }

    fn discovered_builder_from_root(
        &mut self,
        project_root: &Path,
        dir: &Path,
    ) -> Result<ConfigBuilder, CliError> {
        if let Some(builder) = self.discovered_by_dir.get(dir) {
            return Ok(builder.clone());
        }

        let mut builder = if dir == project_root {
            ConfigBuilder::new(&self.defaults)
        } else if let Some(parent) = dir
            .parent()
            .filter(|parent| parent.starts_with(project_root))
        {
            self.discovered_builder_from_root(project_root, parent)?
        } else {
            ConfigBuilder::new(&self.defaults)
        };

        for config in self.directory_config(dir)?.configs {
            builder.apply_sparse(&config);
        }

        self.discovered_by_dir
            .insert(dir.to_path_buf(), builder.clone());
        Ok(builder)
    }

    fn project_root_for_dir(&mut self, dir: &Path) -> Result<PathBuf, CliError> {
        if let Some(root) = self.project_root_by_dir.get(dir) {
            return Ok(root.clone());
        }

        for ancestor in dir.ancestors() {
            if self.is_project_boundary(ancestor)? {
                let root = ancestor.to_path_buf();
                self.project_root_by_dir
                    .insert(dir.to_path_buf(), root.clone());
                return Ok(root);
            }
        }

        let root = self.invocation_root.clone();
        self.project_root_by_dir
            .insert(dir.to_path_buf(), root.clone());
        Ok(root)
    }

    fn is_project_boundary(&mut self, dir: &Path) -> Result<bool, CliError> {
        if let Some(is_boundary) = self.boundary_by_dir.get(dir) {
            return Ok(*is_boundary);
        }

        let is_boundary = has_vcs_marker(dir) || self.has_root_config(dir)?;
        self.boundary_by_dir.insert(dir.to_path_buf(), is_boundary);
        Ok(is_boundary)
    }

    fn has_root_config(&mut self, dir: &Path) -> Result<bool, CliError> {
        for config_path in self.config_paths_for_dir(dir) {
            let config = load_toml_file::<ProjectRootConfig>(&config_path.path)?;
            if config.root == Some(true) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn directory_config(&mut self, dir: &Path) -> Result<DirectoryConfig, CliError> {
        if let Some(config) = self.directory_config_by_dir.get(dir) {
            return Ok(config.clone());
        }

        let mut configs = Vec::new();
        for config_path in self.config_paths_for_dir(dir) {
            configs.push(load_config_at(&config_path.path, &config_path.base_dir)?);
        }

        let config = DirectoryConfig { configs };
        self.directory_config_by_dir
            .insert(dir.to_path_buf(), config.clone());
        Ok(config)
    }

    fn config_paths_for_dir(&mut self, dir: &Path) -> Vec<ConfigPath> {
        if let Some(paths) = self.config_paths_by_dir.get(dir) {
            return paths.clone();
        }

        let paths = config_paths_for_dir(dir)
            .into_iter()
            .filter(|config_path| config_path.path.is_file())
            .collect::<Vec<_>>();
        self.config_paths_by_dir
            .insert(dir.to_path_buf(), paths.clone());
        paths
    }
}

#[derive(Clone, Debug)]
struct ConfigDefaults {
    format: SourcedFormatOptions,
    include: SourceValue<PatternList>,
}

impl ConfigDefaults {
    fn new(invocation_root: &Path) -> Result<Self, CliError> {
        let default = default_config();
        let source = ValueSource::Default;
        Ok(Self {
            format: SourcedFormatOptions::new(default.options, source.clone()),
            include: SourceValue::new(PatternList::new(invocation_root, &default.include)?, source),
        })
    }
}

#[derive(Clone, Debug)]
struct DirectoryConfig {
    configs: Vec<SparseConfig>,
}

#[derive(Clone, Debug)]
struct ConfigBuilder {
    format: SourcedFormatOptions,
    include: SourceValue<PatternList>,
    excludes: Vec<SourceValue<PatternList>>,
}

impl ConfigBuilder {
    fn new(defaults: &ConfigDefaults) -> Self {
        Self {
            format: defaults.format.clone(),
            include: defaults.include.clone(),
            excludes: Vec::new(),
        }
    }

    fn apply_sparse(&mut self, sparse: &SparseConfig) {
        self.format.apply_patch(sparse.format, &sparse.source);
        if let Some(include) = &sparse.include {
            self.include = SourceValue::new(include.clone(), sparse.source.clone());
        }
        if let Some(exclude) = &sparse.exclude {
            self.excludes
                .push(SourceValue::new(exclude.clone(), sparse.source.clone()));
        }
    }

    fn apply_cli_options(&mut self, patch: FormatOptionsPatch) {
        self.format.apply_patch(patch, &ValueSource::Cli);
    }

    fn finish(self) -> ResolvedConfig {
        let (options, format_sources) = self.format.into_parts();
        let SourceValue {
            value: include,
            source: include_source,
        } = self.include;
        let (excludes, exclude_sources) = self
            .excludes
            .into_iter()
            .map(|exclude| (exclude.value, exclude.source))
            .unzip();

        ResolvedConfig {
            options,
            include,
            excludes,
            sources: ResolvedConfigSources {
                format: format_sources,
                include: include_source,
                excludes: exclude_sources,
            },
        }
    }
}

/// The `jolt.toml` (or `.config/jolt.toml`, `.config/jolt/config.toml`) file
/// format: sparse formatting and file-selection overlays that merge across
/// discovered config layers, CLI overrides, and built-in defaults. This same
/// shape is used to load config files, to render the effective resolved
/// config, and to generate the `jolt.toml` JSON schema.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[schemars(title = "Jolt configuration")]
pub(crate) struct FileConfig {
    /// Marks this config as the project root when set to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<bool>,
    /// Formatting options.
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<FormatOptionsPatch>,
    /// File discovery options.
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<FileSelectionPatch>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ProjectRootConfig {
    root: Option<bool>,
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
    let file_config = load_toml_file::<FileConfig>(path)?;
    file_config.into_sparse(path, base_dir)
}

fn load_toml_file<T>(path: &Path) -> Result<T, CliError>
where
    T: DeserializeOwned,
{
    let contents = fs::read_to_string(path).map_err(|error| {
        CliError::new(format!(
            "{}: failed to read config: {error}",
            path.display()
        ))
    })?;
    toml_edit::de::from_str(&contents)
        .map_err(|error| CliError::new(format!("{}: {error}", path.display())))
}

impl FileConfig {
    fn from_default_config(default: DefaultConfig) -> Self {
        Self {
            root: Some(true),
            format: Some(FormatOptionsPatch {
                line_width: Some(default.options.line_width),
                indent_width: Some(default.options.indent_width),
                use_tabs: Some(default.options.use_tabs),
            }),
            files: Some(FileSelectionPatch {
                include: Some(default.include),
                exclude: None,
            }),
        }
    }

    fn into_sparse(self, path: &Path, base_dir: &Path) -> Result<SparseConfig, CliError> {
        let FileConfig { format, files, .. } = self;
        let source = ValueSource::Config(path.to_path_buf());

        let format = format.unwrap_or_default();
        validate_options(format, &path.display().to_string())?;

        let files = files.unwrap_or_default();
        let include = files
            .include
            .map(|patterns| PatternList::new(base_dir, &patterns))
            .transpose()
            .map_err(|error| error.with_source(path))?;
        let exclude = files
            .exclude
            .map(|patterns| PatternList::new(base_dir, &patterns))
            .transpose()
            .map_err(|error| error.with_source(path))?;

        Ok(SparseConfig {
            source,
            format,
            include,
            exclude,
        })
    }
}

fn validate_options(options: FormatOptionsPatch, source: &str) -> Result<(), CliError> {
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

pub(crate) fn discovered_config_paths_for_dir(
    invocation_root: &Path,
    dir: &Path,
) -> Result<Vec<PathBuf>, CliError> {
    let mut graph = ConfigGraph::new(
        invocation_root,
        invocation_root.to_path_buf(),
        FormatOptionsPatch::default(),
        &[],
        &[],
        None,
        false,
    )?;
    graph.discovered_config_paths_for_dir(dir)
}

pub(crate) fn render_resolved_config(
    config: &ResolvedConfig,
    target_file: Option<&Path>,
) -> Result<String, CliError> {
    let exclude_patterns = config
        .excludes
        .iter()
        .flat_map(|exclude| exclude.patterns().iter().cloned())
        .collect::<Vec<_>>();

    let render_config = FileConfig {
        root: None,
        format: Some(FormatOptionsPatch {
            line_width: Some(config.options.line_width),
            indent_width: Some(config.options.indent_width),
            use_tabs: Some(config.options.use_tabs),
        }),
        files: Some(FileSelectionPatch {
            include: Some(config.include.patterns().to_vec()),
            exclude: (!exclude_patterns.is_empty()).then_some(exclude_patterns),
        }),
    };
    let toml = toml_edit::ser::to_string_pretty(&render_config)
        .map_err(|error| CliError::new(format!("failed to serialize resolved config: {error}")))?;
    let mut document = toml
        .parse::<DocumentMut>()
        .map_err(|error| CliError::new(format!("failed to parse resolved config: {error}")))?;

    if let Some(format) = document["format"].as_table_mut() {
        set_key_source_comment(format, "line-width", &[&config.sources.format.line_width]);
        set_key_source_comment(
            format,
            "indent-width",
            &[&config.sources.format.indent_width],
        );
        set_key_source_comment(format, "use-tabs", &[&config.sources.format.use_tabs]);
    }
    if let Some(files) = document["files"].as_table_mut() {
        set_key_source_comment(files, "include", &[&config.sources.include]);
        let exclude_sources = config
            .sources
            .excludes
            .iter()
            .collect::<Vec<&ValueSource>>();
        if !exclude_sources.is_empty() {
            set_key_source_comment(files, "exclude", &exclude_sources);
        }
    }

    if let Some(path) = target_file {
        let status = if config.matches_path(path) {
            "selected"
        } else {
            "not selected"
        };
        document.set_trailing(format!("\n# target {} is {status}\n", path.display()));
    }

    Ok(document.to_string())
}

fn set_key_source_comment(table: &mut toml_edit::Table, key: &str, sources: &[&ValueSource]) {
    let Some(mut key) = table.key_mut(key) else {
        return;
    };

    let comment = source_comment(sources);
    key.leaf_decor_mut().set_prefix(comment);
}

fn source_comment(sources: &[&ValueSource]) -> String {
    // Preserve first-seen output order while bounding membership checks. For
    // `s` sources whose paths contain at most `p` platform units, lookup and
    // rendering are O(s log s * p) time with O(s) borrowed set entries.
    let mut seen = BTreeSet::new();
    let mut comment = String::new();
    for &source in sources {
        if seen.insert(source) {
            writeln!(&mut comment, "# from {}", source.label())
                .expect("writing to a String cannot fail");
        }
    }
    comment
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

fn has_vcs_marker(dir: &Path) -> bool {
    VCS_MARKERS.iter().any(|marker| dir.join(marker).exists())
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
