//! dprint plugin handler for Jolt.

use std::path::Path;

use dprint_core::plugins::{FormatError, FormatResult};
#[cfg(feature = "wasm")]
use dprint_core::{
    configuration::{ConfigKeyMap, GlobalConfiguration},
    plugins::{
        CheckConfigUpdatesMessage, ConfigChange, PluginInfo, PluginResolveConfigurationResult,
    },
};
use jolt_formatter::{Diagnostic, FormatOptions, Language, LineIndex, OnSyntaxError, Sink};

#[cfg(feature = "wasm")]
use crate::configuration;

/// dprint plugin handler that delegates formatting to `jolt_formatter`.
#[derive(Debug, Default)]
pub(crate) struct JoltDprintPlugin;

impl JoltDprintPlugin {
    /// Creates a Jolt dprint plugin handler.
    #[cfg(all(feature = "wasm", target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) const fn new() -> Self {
        Self
    }

    /// Formats a dprint request payload using Jolt's shared formatter facade.
    ///
    /// # Errors
    ///
    /// Returns an error when the file is not UTF-8, the path is not a supported
    /// Jolt language, or the core formatter blocks without producing output.
    fn format_file(file_path: &Path, file_bytes: &[u8], options: FormatOptions) -> FormatResult {
        let source = std::str::from_utf8(file_bytes).map_err(|error| {
            FormatError::from(format!("Jolt formatter requires UTF-8 input: {error}"))
        })?;
        let language = language_for_path(file_path)?;
        let mut sink = DprintFormatSink::default();
        match jolt_formatter::format(source, language, &options, OnSyntaxError::Refuse, &mut sink) {
            jolt_formatter::FormatResult::Formatted { halted: false } => {}
            jolt_formatter::FormatResult::Formatted { halted: true } => {
                return Err(FormatError::from(
                    "Jolt formatter halted before producing complete output",
                ));
            }
            jolt_formatter::FormatResult::Refused(diagnostics) => {
                let line_index = LineIndex::new(source);
                return Err(FormatError::from(format_diagnostic(
                    source,
                    &line_index,
                    &diagnostics[0],
                )));
            }
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
        Self::format_file(request.file_path, &request.file_bytes, *request.config)
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

impl Sink for DprintFormatSink {
    fn write(&mut self, text: &str) -> std::ops::ControlFlow<()> {
        self.bytes.extend_from_slice(text.as_bytes());
        std::ops::ControlFlow::Continue(())
    }
}

fn language_for_path(file_path: &Path) -> Result<Language, FormatError> {
    match Language::from_path(file_path) {
        Some(language) => Ok(language),
        None => match file_path
            .extension()
            .and_then(|extension| extension.to_str())
        {
            Some(extension) => Err(FormatError::from(format!(
                "Jolt dprint plugin does not support '.{extension}' files"
            ))),
            None => Err(FormatError::from(
                "Jolt dprint plugin requires a supported file extension",
            )),
        },
    }
}

fn format_diagnostic(source: &str, line_index: &LineIndex, diagnostic: &Diagnostic) -> String {
    let range = diagnostic.range;
    let location = line_index.line_col(range.start());
    let mut text = format!(
        "code={} message={} line={} column={} range={range}",
        diagnostic.code,
        diagnostic.message,
        location.line + 1,
        location.col + 1
    );
    if range.start().get() > source.len() {
        text.push_str(" location=out-of-bounds");
    }
    text
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use dprint_core::plugins::FormatError;
    use jolt_formatter::FormatOptions;

    use super::JoltDprintPlugin;

    #[test]
    fn recovered_parse_returns_an_error() {
        let error = format_java("class Foo{", FormatOptions::default())
            .expect_err("format should reject syntax errors");

        assert!(error.to_string().contains("java.parse.expected_syntax"));
    }

    #[test]
    fn unsupported_associated_paths_return_errors_without_formatted_bytes() {
        let error = JoltDprintPlugin::format_file(
            Path::new("Associated.scala"),
            b"fun main() {}",
            FormatOptions::default(),
        )
        .expect_err("format should fail");

        assert!(error.to_string().contains("does not support '.scala'"));
    }

    #[test]
    fn invalid_utf8_returns_errors_without_formatted_bytes() {
        let error = JoltDprintPlugin::format_file(
            Path::new("Broken.java"),
            &[0xff],
            FormatOptions::default(),
        )
        .expect_err("format should fail");

        assert!(error.to_string().contains("requires UTF-8 input"));
    }

    fn format_java(source: &str, options: FormatOptions) -> Result<Option<Vec<u8>>, FormatError> {
        JoltDprintPlugin::format_file(Path::new("A.java"), source.as_bytes(), options)
    }
}
