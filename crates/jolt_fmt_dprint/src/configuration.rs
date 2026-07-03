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
pub fn resolve_config(
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
pub fn file_matching_info() -> FileMatchingInfo {
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
