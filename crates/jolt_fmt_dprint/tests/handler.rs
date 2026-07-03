use std::path::Path;

use dprint_core::plugins::FormatError;
use jolt_fmt_core::FormatOptions;
use jolt_fmt_dprint::JoltDprintPlugin;

#[test]
fn plugin_metadata_matches_initial_dprint_contract() {
    let plugin = JoltDprintPlugin::new();
    let info = plugin.jolt_plugin_info();

    assert_eq!(info.name, "jolt_fmt_dprint");
    assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(info.config_key, "jolt");
    assert!(!info.help_url.is_empty());
    assert!(info.config_schema_url.is_empty());
    assert_eq!(info.update_url, None);
}

#[test]
fn java_requests_call_the_java_formatter_and_return_changed_bytes() {
    let result =
        format_java("class A {}", FormatOptions::default()).expect("format should succeed");

    assert_eq!(result.as_deref(), Some(b"class A {\n}\n".as_slice()));
}

#[test]
fn already_formatted_java_returns_no_change() {
    let result =
        format_java("class A {\n}\n", FormatOptions::default()).expect("format should succeed");

    assert_eq!(result, None);
}

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
            Path::new("Associated.kt"),
            b"fun main() {}",
            &FormatOptions::default(),
        )
        .expect_err("format should fail");

    assert!(error.to_string().contains("does not support '.kt'"));
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
