//! The dprint Wasm plugin. Natively it builds only for its tests.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use std::{fmt::Write as _, path::Path};

use dprint_core::{
    configuration::{ConfigKeyMap, ConfigurationDiagnostic, GlobalConfiguration},
    plugins::{
        CheckConfigUpdatesMessage, ConfigChange, FileMatchingInfo, FormatError, FormatResult,
        PluginInfo, PluginResolveConfigurationResult, SyncFormatRequest, SyncHostFormatRequest,
        SyncPluginHandler,
    },
};
use jolt_config::FormatConfig;
use jolt_formatter::{FormatOptions, Language, LineIndex, OnSyntaxError};
use serde_json::Value;

#[derive(Default)]
struct JoltPlugin;

#[cfg(target_arch = "wasm32")]
dprint_core::generate_plugin_code!(JoltPlugin, JoltPlugin, FormatOptions);

impl SyncPluginHandler<FormatOptions> for JoltPlugin {
    fn resolve_config(
        &mut self,
        config: ConfigKeyMap,
        global_config: &GlobalConfiguration,
    ) -> PluginResolveConfigurationResult<FormatOptions> {
        resolve_config(config, global_config)
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
        request: SyncFormatRequest<FormatOptions>,
        _format_with_host: impl FnMut(SyncHostFormatRequest) -> FormatResult,
    ) -> FormatResult {
        format_file(request.file_path, &request.file_bytes, *request.config)
    }
}

/// The plugin's options layered over dprint's global options.
fn resolve_config(
    config: ConfigKeyMap,
    global_config: &GlobalConfiguration,
) -> PluginResolveConfigurationResult<FormatOptions> {
    let mut diagnostics = Vec::new();
    let Ok(Value::Object(globals)) = serde_json::to_value(global_config) else {
        unreachable!("dprint's global configuration serializes to an object");
    };
    // Jolt always writes `\n`.
    let globals = globals.into_iter().filter(|(key, _)| key != "newLineKind");
    let plugin = config.into_iter().map(|(key, value)| {
        let value = serde_json::to_value(value).expect("a config value serializes");
        (key, value)
    });
    let options = parse(plugin, &mut diagnostics).or(parse(globals, &mut diagnostics));
    PluginResolveConfigurationResult {
        file_matching: FileMatchingInfo {
            file_extensions: vec!["java".to_owned()],
            file_names: Vec::new(),
        },
        diagnostics,
        config: options.resolve(),
    }
}

/// Parses options one key at a time, so each invalid key gets its own
/// diagnostic and the valid ones still apply. `null` leaves a key unset.
fn parse(
    entries: impl IntoIterator<Item = (String, Value)>,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) -> FormatConfig {
    let mut config = FormatConfig::default();
    for (key, value) in entries {
        if value.is_null() {
            continue;
        }
        let entry = Value::Object([(key.clone(), value)].into_iter().collect());
        match serde_json::from_value::<FormatConfig>(entry) {
            Ok(parsed) => config = parsed.or(config),
            Err(error) => diagnostics.push(ConfigurationDiagnostic {
                property_name: key,
                message: error.to_string(),
            }),
        }
    }
    config
}

/// Formats a file, failing on syntax errors with one `line:column: code:
/// message` line per diagnostic.
fn format_file(path: &Path, bytes: &[u8], options: FormatOptions) -> FormatResult {
    let source = std::str::from_utf8(bytes)
        .map_err(|error| FormatError::from(format!("Jolt requires UTF-8 input: {error}")))?;
    let language = Language::from_path(path)
        .ok_or_else(|| FormatError::from(format!("Jolt does not format {}", path.display())))?;
    let mut output = String::new();
    match jolt_formatter::format(
        source,
        language,
        &options,
        OnSyntaxError::Refuse,
        &mut output,
    ) {
        jolt_formatter::FormatResult::Formatted { .. } => {
            Ok((output != source).then(|| output.into_bytes()))
        }
        jolt_formatter::FormatResult::Refused(diagnostics) => {
            let lines = LineIndex::new(source);
            let mut message = String::new();
            for diagnostic in diagnostics {
                let at = lines.line_col(diagnostic.range.start());
                let _ = writeln!(
                    message,
                    "{}:{}: {}: {}",
                    at.line + 1,
                    at.col + 1,
                    diagnostic.code,
                    diagnostic.message
                );
            }
            Err(FormatError::from(message.trim_end().to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use dprint_core::configuration::{ConfigKeyMap, ConfigKeyValue, GlobalConfiguration};
    use jolt_formatter::FormatOptions;

    use super::resolve_config;

    fn global() -> GlobalConfiguration {
        GlobalConfiguration {
            line_width: Some(100),
            indent_width: Some(4),
            use_tabs: Some(true),
            new_line_kind: None,
        }
    }

    #[test]
    fn plugin_options_override_global_options_and_null_keeps_them() {
        let config = ConfigKeyMap::from_iter([
            ("lineWidth".to_owned(), ConfigKeyValue::from_i32(90)),
            ("indentWidth".to_owned(), ConfigKeyValue::Null),
        ]);
        let result = resolve_config(config, &global());
        assert!(result.diagnostics.is_empty());
        let expected = FormatOptions {
            line_width: 90,
            indent_width: 4,
            use_tabs: true,
        };
        assert_eq!(result.config, expected);
    }

    #[test]
    fn invalid_options_get_diagnostics_and_fall_back() {
        let config = ConfigKeyMap::from_iter([
            ("lineWidth".to_owned(), ConfigKeyValue::from_i32(0)),
            ("useTabs".to_owned(), ConfigKeyValue::from_i32(1)),
            ("surprise".to_owned(), ConfigKeyValue::from_bool(true)),
        ]);
        let result = resolve_config(config, &global());
        let properties: Vec<_> = result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.property_name.as_str())
            .collect();
        assert_eq!(properties, ["lineWidth", "useTabs", "surprise"]);
        assert_eq!(result.config.line_width, 100);
    }
}
