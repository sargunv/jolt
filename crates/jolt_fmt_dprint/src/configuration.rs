//! dprint configuration resolution for Jolt.

use dprint_core::{
    configuration::{
        ConfigKeyMap, ConfigKeyValue, ConfigurationDiagnostic, GlobalConfiguration,
        get_unknown_property_diagnostics,
    },
    plugins::{FileMatchingInfo, PluginResolveConfigurationResult},
};
use jolt_fmt_core::FormatOptions;

/// Resolves dprint global and plugin configuration into Jolt's shared options.
#[must_use]
pub(crate) fn resolve_config(
    mut config: ConfigKeyMap,
    global_config: &GlobalConfiguration,
) -> PluginResolveConfigurationResult<FormatOptions> {
    let mut diagnostics = Vec::new();
    let mut options = FormatOptions::default();

    apply_global_config(&mut options, global_config, &mut diagnostics);
    apply_plugin_config(&mut options, &mut config, &mut diagnostics);
    diagnostics.extend(get_unknown_property_diagnostics(config));

    PluginResolveConfigurationResult {
        file_matching: file_matching_info(),
        diagnostics,
        config: options,
    }
}

/// Returns the file matching declaration for the initial Jolt plugin.
#[must_use]
fn file_matching_info() -> FileMatchingInfo {
    FileMatchingInfo {
        file_extensions: vec!["java".to_owned()],
        file_names: Vec::new(),
    }
}

fn apply_global_config(
    options: &mut FormatOptions,
    global_config: &GlobalConfiguration,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
    if let Some(line_width) = global_config.line_width {
        apply_u32_as_u16(options, "lineWidth", line_width, diagnostics);
    }
    if let Some(indent_width) = global_config.indent_width {
        apply_u8(options, "indentWidth", indent_width, diagnostics);
    }
    if let Some(use_tabs) = global_config.use_tabs {
        options.use_tabs = use_tabs;
    }
}

fn apply_plugin_config(
    options: &mut FormatOptions,
    config: &mut ConfigKeyMap,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
    if let Some(value) = config.shift_remove("lineWidth")
        && let Some(line_width) = read_i32("lineWidth", value, diagnostics)
    {
        apply_i32_as_u16(options, "lineWidth", line_width, diagnostics);
    }
    if let Some(value) = config.shift_remove("indentWidth")
        && let Some(indent_width) = read_i32("indentWidth", value, diagnostics)
    {
        apply_i32_as_u8(options, "indentWidth", indent_width, diagnostics);
    }
    if let Some(value) = config.shift_remove("useTabs")
        && let Some(use_tabs) = read_bool("useTabs", value, diagnostics)
    {
        options.use_tabs = use_tabs;
    }
}

fn read_i32(
    property_name: &str,
    value: ConfigKeyValue,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) -> Option<i32> {
    match value {
        ConfigKeyValue::Number(value) => Some(value),
        ConfigKeyValue::String(value) => match value.parse::<i32>() {
            Ok(value) => Some(value),
            Err(error) => {
                push_diagnostic(property_name, error.to_string(), diagnostics);
                None
            }
        },
        ConfigKeyValue::Null => None,
        ConfigKeyValue::Bool(_) | ConfigKeyValue::Array(_) | ConfigKeyValue::Object(_) => {
            push_diagnostic(
                property_name,
                "Expected an integer configuration value.",
                diagnostics,
            );
            None
        }
    }
}

fn read_bool(
    property_name: &str,
    value: ConfigKeyValue,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) -> Option<bool> {
    match value {
        ConfigKeyValue::Bool(value) => Some(value),
        ConfigKeyValue::String(value) => match value.parse::<bool>() {
            Ok(value) => Some(value),
            Err(error) => {
                push_diagnostic(property_name, error.to_string(), diagnostics);
                None
            }
        },
        ConfigKeyValue::Null => None,
        ConfigKeyValue::Number(_) | ConfigKeyValue::Array(_) | ConfigKeyValue::Object(_) => {
            push_diagnostic(
                property_name,
                "Expected a boolean configuration value.",
                diagnostics,
            );
            None
        }
    }
}

fn apply_u32_as_u16(
    options: &mut FormatOptions,
    property_name: &str,
    value: u32,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
    match u16::try_from(value) {
        Ok(value) => apply_u16(options, property_name, value, diagnostics),
        Err(_) => push_diagnostic(
            property_name,
            "Value is outside the supported range for Jolt format options.",
            diagnostics,
        ),
    }
}

fn apply_i32_as_u16(
    options: &mut FormatOptions,
    property_name: &str,
    value: i32,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
    match u16::try_from(value) {
        Ok(value) => apply_u16(options, property_name, value, diagnostics),
        Err(_) => push_diagnostic(
            property_name,
            "Value is outside the supported range for Jolt format options.",
            diagnostics,
        ),
    }
}

fn apply_i32_as_u8(
    options: &mut FormatOptions,
    property_name: &str,
    value: i32,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
    match u8::try_from(value) {
        Ok(value) => apply_u8(options, property_name, value, diagnostics),
        Err(_) => push_diagnostic(
            property_name,
            "Value is outside the supported range for Jolt format options.",
            diagnostics,
        ),
    }
}

fn apply_u16(
    options: &mut FormatOptions,
    property_name: &str,
    value: u16,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
    if value == 0 {
        push_diagnostic(
            property_name,
            "Value must be greater than zero.",
            diagnostics,
        );
    } else {
        options.line_width = value;
    }
}

fn apply_u8(
    options: &mut FormatOptions,
    property_name: &str,
    value: u8,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
    if value == 0 {
        push_diagnostic(
            property_name,
            "Value must be greater than zero.",
            diagnostics,
        );
    } else {
        options.indent_width = value;
    }
}

fn push_diagnostic(
    property_name: &str,
    message: impl Into<String>,
    diagnostics: &mut Vec<ConfigurationDiagnostic>,
) {
    diagnostics.push(ConfigurationDiagnostic {
        property_name: property_name.to_owned(),
        message: message.into(),
    });
}

#[cfg(test)]
mod tests {
    use dprint_core::configuration::{ConfigKeyMap, ConfigKeyValue, GlobalConfiguration};
    use jolt_fmt_core::FormatOptions;

    use super::resolve_config;

    #[test]
    fn dprint_global_config_maps_to_format_options() {
        let global = GlobalConfiguration {
            line_width: Some(100),
            indent_width: Some(4),
            use_tabs: Some(true),
            new_line_kind: None,
        };

        let result = resolve_config(ConfigKeyMap::new(), &global);

        assert_eq!(
            result.config,
            FormatOptions {
                line_width: 100,
                indent_width: 4,
                use_tabs: true,
            }
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn jolt_plugin_config_overrides_global_config() {
        let global = GlobalConfiguration {
            line_width: Some(100),
            indent_width: Some(4),
            use_tabs: Some(true),
            new_line_kind: None,
        };
        let config = config_map([
            ("lineWidth", ConfigKeyValue::from_i32(90)),
            ("indentWidth", ConfigKeyValue::from_i32(3)),
            ("useTabs", ConfigKeyValue::from_bool(false)),
        ]);

        let result = resolve_config(config, &global);

        assert_eq!(
            result.config,
            FormatOptions {
                line_width: 90,
                indent_width: 3,
                use_tabs: false,
            }
        );
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn unknown_properties_produce_configuration_diagnostics() {
        let config = config_map([("surprise", ConfigKeyValue::from_bool(true))]);

        let result = resolve_config(config, &GlobalConfiguration::default());

        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].property_name, "surprise");
        assert!(result.diagnostics[0].message.contains("Unknown property"));
    }

    #[test]
    fn zero_numeric_values_produce_configuration_diagnostics() {
        let config = config_map([
            ("lineWidth", ConfigKeyValue::from_i32(0)),
            ("indentWidth", ConfigKeyValue::from_i32(0)),
        ]);

        let result = resolve_config(config, &GlobalConfiguration::default());

        assert_eq!(diagnostic_properties(&result), ["lineWidth", "indentWidth"]);
        assert_eq!(result.config, FormatOptions::default());
    }

    #[test]
    fn out_of_range_numeric_values_produce_configuration_diagnostics() {
        let config = config_map([
            (
                "lineWidth",
                ConfigKeyValue::from_i32(i32::from(u16::MAX) + 1),
            ),
            (
                "indentWidth",
                ConfigKeyValue::from_i32(i32::from(u8::MAX) + 1),
            ),
        ]);

        let result = resolve_config(config, &GlobalConfiguration::default());

        assert_eq!(diagnostic_properties(&result), ["lineWidth", "indentWidth"]);
        assert_eq!(result.config, FormatOptions::default());
    }

    #[test]
    fn invalid_global_numeric_values_produce_configuration_diagnostics() {
        let global = GlobalConfiguration {
            line_width: Some(u32::from(u16::MAX) + 1),
            indent_width: Some(0),
            use_tabs: None,
            new_line_kind: None,
        };

        let result = resolve_config(ConfigKeyMap::new(), &global);

        assert_eq!(diagnostic_properties(&result), ["lineWidth", "indentWidth"]);
        assert_eq!(result.config, FormatOptions::default());
    }

    fn config_map<const N: usize>(entries: [(&str, ConfigKeyValue); N]) -> ConfigKeyMap {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect()
    }

    fn diagnostic_properties(
        result: &dprint_core::plugins::PluginResolveConfigurationResult<FormatOptions>,
    ) -> Vec<&str> {
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.property_name.as_str())
            .collect()
    }
}
