//! dprint plugin handler for Jolt.

use std::{convert::Infallible, fmt::Write as _, path::Path};

use dprint_core::plugins::{FormatError, FormatResult};
#[cfg(feature = "wasm")]
use dprint_core::{
    configuration::{ConfigKeyMap, GlobalConfiguration},
    plugins::{
        CheckConfigUpdatesMessage, ConfigChange, PluginInfo, PluginResolveConfigurationResult,
    },
};
use jolt_diagnostics::{Diagnostic, DiagnosticStage, Severity};
use jolt_fmt_ir::{RenderControl, RenderSink};
use jolt_formatter::{FormatOptions, FormatSinkResult, Language, format_source_to_sink};
use jolt_text::LineIndex;

#[cfg(feature = "wasm")]
use crate::configuration;

/// dprint plugin handler that delegates formatting to `jolt_formatter`.
#[derive(Debug, Default)]
pub(crate) struct JoltDprintPlugin;

impl JoltDprintPlugin {
    /// Creates a Jolt dprint plugin handler.
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Formats a dprint request payload using Jolt's shared formatter facade.
    ///
    /// # Errors
    ///
    /// Returns an error when the file is not UTF-8, the path is not a supported
    /// Jolt language, or the core formatter blocks without producing output.
    fn format_file(
        &self,
        file_path: &Path,
        file_bytes: &[u8],
        options: &FormatOptions,
    ) -> FormatResult {
        let source = std::str::from_utf8(file_bytes).map_err(|error| {
            FormatError::from(format!("Jolt formatter requires UTF-8 input: {error}"))
        })?;
        let language = language_for_path(file_path)?;

        let mut sink = DprintFormatSink::default();
        let result = format_source_to_sink(source, language, options, &mut sink);
        match result {
            FormatSinkResult::Complete => {}
            FormatSinkResult::Halted => {
                return Err(FormatError::from(
                    "Jolt formatter halted before producing complete output",
                ));
            }
            FormatSinkResult::Blocked { diagnostics } => {
                return Err(FormatError::from(format_blocked_diagnostics(
                    source,
                    &diagnostics,
                )));
            }
            FormatSinkResult::SinkError { error } => match error {},
        }

        let formatted = sink.into_bytes();
        if formatted == file_bytes {
            Ok(None)
        } else {
            Ok(Some(formatted))
        }
    }
}

#[cfg(feature = "wasm")]
impl dprint_core::plugins::SyncPluginHandler<FormatOptions> for JoltDprintPlugin {
    fn resolve_config(
        &mut self,
        config: ConfigKeyMap,
        global_config: &GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<FormatOptions> {
        configuration::resolve_config(config, global_config)
    }

    fn plugin_info(&mut self) -> PluginInfo {
        PluginInfo {
            name: env!("CARGO_PKG_NAME").to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            config_key: "jolt".to_owned(),
            help_url: env!("CARGO_PKG_REPOSITORY").to_owned(),
            config_schema_url: format!(
                "{}/releases/download/{}/dprint-schema.json",
                env!("CARGO_PKG_REPOSITORY"),
                env!("CARGO_PKG_VERSION"),
            ),
            update_url: None,
        }
    }

    fn license_text(&mut self) -> String {
        env!("CARGO_PKG_LICENSE").to_owned()
    }

    fn check_config_updates(
        &self,
        _message: CheckConfigUpdatesMessage,
    ) -> Result<Vec<ConfigChange>, FormatError> {
        Ok(Vec::new())
    }

    fn format(
        &mut self,
        request: dprint_core::plugins::SyncFormatRequest<FormatOptions>,
        _format_with_host: impl FnMut(dprint_core::plugins::SyncHostFormatRequest) -> FormatResult,
    ) -> FormatResult {
        self.format_file(request.file_path, &request.file_bytes, request.config)
    }
}

#[derive(Default)]
struct DprintFormatSink {
    bytes: Vec<u8>,
}

impl DprintFormatSink {
    fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl RenderSink for DprintFormatSink {
    type Error = Infallible;

    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
        self.bytes.extend_from_slice(text.as_bytes());
        Ok(RenderControl::Continue)
    }
}

fn language_for_path(file_path: &Path) -> Result<Language, FormatError> {
    match file_path
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("java") => Ok(Language::Java),
        Some("kt" | "kts") => Ok(Language::Kotlin),
        Some(extension) => Err(FormatError::from(format!(
            "Jolt dprint plugin does not support '.{extension}' files"
        ))),
        None => Err(FormatError::from(
            "Jolt dprint plugin requires a supported file extension",
        )),
    }
}

fn format_blocked_diagnostics(source: &str, diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "Jolt formatter blocked without diagnostics.".to_owned();
    }

    let line_index = LineIndex::new(source);
    let mut text = String::new();
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        if index > 0 {
            text.push('\n');
        }
        text.push_str(&format_diagnostic(source, &line_index, diagnostic));
    }
    text
}

fn format_diagnostic(source: &str, line_index: &LineIndex, diagnostic: &Diagnostic) -> String {
    let mut text = format!(
        "code={} severity={} stage={} message={}",
        diagnostic.code.as_str(),
        severity_name(diagnostic.severity),
        stage_name(diagnostic.stage),
        diagnostic.message
    );

    if let Some(range) = diagnostic.range {
        let location = line_index.line_col(range.start());
        write!(
            text,
            " line={} column={} range={range}",
            location.line + 1,
            usize::from(location.column) + 1
        )
        .expect("writing to a String should not fail");
        if range.start().get() > source.len() {
            text.push_str(" location=out-of-bounds");
        }
    }

    text
}

const fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::InternalError => "internal-error",
        Severity::Error => "error",
    }
}

const fn stage_name(stage: DiagnosticStage) -> &'static str {
    match stage {
        DiagnosticStage::Lexer => "lexer",
        DiagnosticStage::Parser => "parser",
        DiagnosticStage::Formatter => "formatter",
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use dprint_core::plugins::FormatError;
    use jolt_formatter::FormatOptions;

    use super::JoltDprintPlugin;

    #[test]
    fn parse_errors_return_dprint_errors_without_formatted_bytes() {
        let error = format_java("class", FormatOptions::default()).expect_err("format should fail");
        let message = error.to_string();

        assert!(message.contains("code=java.parse."));
        assert!(message.contains("severity=error"));
        assert!(message.contains("stage=parser"));
        assert!(message.contains("message="));
        assert!(message.contains("line=1"));
        assert!(message.contains("column="));
    }

    #[test]
    fn unsupported_associated_paths_return_errors_without_formatted_bytes() {
        let plugin = JoltDprintPlugin::new();
        let error = plugin
            .format_file(
                Path::new("Associated.scala"),
                b"fun main() {}",
                &FormatOptions::default(),
            )
            .expect_err("format should fail");

        assert!(error.to_string().contains("does not support '.scala'"));
    }

    #[test]
    fn invalid_utf8_returns_errors_without_formatted_bytes() {
        let plugin = JoltDprintPlugin::new();
        let error = plugin
            .format_file(Path::new("Broken.java"), &[0xff], &FormatOptions::default())
            .expect_err("format should fail");

        assert!(error.to_string().contains("requires UTF-8 input"));
    }

    fn format_java(source: &str, options: FormatOptions) -> Result<Option<Vec<u8>>, FormatError> {
        JoltDprintPlugin::new().format_file(Path::new("A.java"), source.as_bytes(), &options)
    }
}
