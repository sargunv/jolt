use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

use jolt_formatter::{FormatOptions, Language};

use crate::error::CliError;

use super::config::ConfigGraph;

#[derive(Clone, Debug)]
pub(crate) struct CandidateFile {
    pub(crate) path: PathBuf,
    pub(crate) language: Language,
    pub(crate) options: FormatOptions,
}

pub(crate) fn discover_files(
    root: &Path,
    config_graph: &mut ConfigGraph,
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
        let config = config_graph.resolve_for_dir(parent)?;

        if !config.matches_path(&path) {
            continue;
        }

        candidates.push(CandidateFile {
            path,
            language,
            options: config.options,
        });
    }

    // Cost model: `n` discovered paths are stably sorted with O(n log n)
    // comparisons, each bounded by the longer path (`p` platform units), for
    // O(n log n * p) time and O(n) storage. This is the deterministic
    // file-discovery ordering; it does not participate in formatter layout.
    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(candidates)
}

pub(crate) fn detect_language(path: &Path) -> Option<Language> {
    Language::from_path(path)
}
