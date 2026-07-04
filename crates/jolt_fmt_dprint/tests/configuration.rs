use dprint_core::configuration::{ConfigKeyMap, ConfigKeyValue, GlobalConfiguration};
use jolt_fmt_core::FormatOptions;
use jolt_fmt_dprint::configuration::resolve_config;

#[test]
fn plugin_config_uses_jolt_defaults_without_global_or_plugin_values() {
    let result = resolve_config(ConfigKeyMap::new(), &GlobalConfiguration::default());

    assert_eq!(result.config, FormatOptions::default());
    assert!(result.diagnostics.is_empty());
}

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
fn supported_config_keys_use_camel_case_dprint_names() {
    let config = config_map([
        ("lineWidth", ConfigKeyValue::from_i32(88)),
        ("indentWidth", ConfigKeyValue::from_i32(6)),
        ("useTabs", ConfigKeyValue::from_bool(true)),
    ]);

    let result = resolve_config(config, &GlobalConfiguration::default());

    assert_eq!(result.config.line_width, 88);
    assert_eq!(result.config.indent_width, 6);
    assert!(result.config.use_tabs);
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

#[test]
fn file_matching_registers_only_java_extensions() {
    let result = resolve_config(ConfigKeyMap::new(), &GlobalConfiguration::default());

    assert_eq!(result.file_matching.file_extensions, ["java"]);
    assert!(result.file_matching.file_names.is_empty());
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
