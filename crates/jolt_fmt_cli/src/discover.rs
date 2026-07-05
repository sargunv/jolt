use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use jolt_fmt_core::{FormatOptions, Language};

use crate::config::{CliError, ConfigResolver, ResolvedConfig};

#[derive(Clone, Debug)]
pub(crate) struct CandidateFile {
    pub(crate) path: PathBuf,
    pub(crate) language: Language,
    pub(crate) options: FormatOptions,
}

pub(crate) fn discover_files(
    root: &Path,
    resolver: &mut ConfigResolver,
) -> Result<Vec<CandidateFile>, CliError> {
    let mut candidates = Vec::new();

    for entry in WalkBuilder::new(root).require_git(false).build() {
        let entry = entry.map_err(|error| CliError::new(format!("{error}")))?;
        if !entry
            .file_type()
            .is_some_and(|file_type| file_type.is_file())
        {
            continue;
        }

        let path = entry.into_path();
        let Some(language) = detect_language(&path) else {
            continue;
        };

        let parent = path.parent().unwrap_or(root);
        let config = resolver.resolve_for_dir(parent)?;

        if !matches_selection(&path, &config) {
            continue;
        }

        candidates.push(CandidateFile {
            path,
            language,
            options: config.options,
        });
    }

    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(candidates)
}

fn matches_selection(path: &Path, config: &ResolvedConfig) -> bool {
    config.include.matches(path) && !config.excludes.iter().any(|exclude| exclude.matches(path))
}

pub(crate) fn detect_language(path: &Path) -> Option<Language> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("java") => Some(Language::Java),
        _ => None,
    }
}
