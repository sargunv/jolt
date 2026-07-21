use std::io;

use schemars::{
    JsonSchema,
    schema::{RootSchema, Schema, SchemaObject},
    schema_for,
};
use serde::Serialize;

use crate::fmt::config::FileConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SchemaKind {
    Jolt,
    Dprint,
}

pub(crate) fn write_schema(kind: SchemaKind, writer: &mut impl io::Write) -> io::Result<()> {
    let schema = match kind {
        SchemaKind::Jolt => jolt_schema(),
        SchemaKind::Dprint => dprint_schema(),
    };
    let json = serde_json::to_string_pretty(&schema)
        .expect("configuration schema should serialize to JSON");
    writer.write_all(json.as_bytes())?;
    writer.write_all(b"\n")
}

/// Outer schema shape for dprint's `dprint.jsonc`: plugin options nested under
/// the `jolt` config key, with all other dprint global keys permitted.
///
/// This mirrors `crate::fmt::config::FormatOptionsPatch`, but uses camelCase
/// field names to match dprint's plugin config convention rather than the
/// kebab-case `jolt.toml` convention.
#[derive(Serialize, JsonSchema)]
#[schemars(title = "dprint jolt plugin configuration")]
struct DprintJoltConfig {
    /// Jolt-specific options; dprint global options (`lineWidth`, etc.) are
    /// applied as defaults when these are absent.
    jolt: Option<DprintFormatConfig>,
}

/// Jolt formatter options under dprint's plugin config key.
///
/// dprint accepts `null` as an explicit unset value for plugin options; the
/// runtime treats that the same as an absent key after applying global defaults.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct DprintFormatConfig {
    /// Preferred maximum rendered line width.
    #[schemars(range(min = 1, max = 65535))]
    line_width: Option<u16>,
    /// Number of spaces per indentation level when `useTabs` is false.
    #[schemars(range(min = 1, max = 255))]
    indent_width: Option<u8>,
    /// Whether indentation should use tabs instead of spaces.
    use_tabs: Option<bool>,
}

fn dprint_schema() -> RootSchema {
    let mut schema = schema_for!(DprintJoltConfig);
    clear_required(&mut schema.schema);
    allow_additional_properties(&mut schema.schema);
    for def in schema.definitions.values_mut() {
        if let Schema::Object(obj) = def {
            clear_required(obj);
            deny_additional_properties(obj);
        }
    }
    schema
}

fn jolt_schema() -> RootSchema {
    let mut schema = schema_for!(FileConfig);
    clear_required(&mut schema.schema);
    deny_additional_properties(&mut schema.schema);
    for def in schema.definitions.values_mut() {
        if let Schema::Object(obj) = def {
            clear_required(obj);
            deny_additional_properties(obj);
        }
    }
    schema
}

/// Clears the `required` list so users can omit any field and rely on discovered
/// config layers, dprint globals, or built-in defaults.
fn clear_required(obj: &mut SchemaObject) {
    if let Some(object_validation) = obj.object.as_mut() {
        object_validation.required.clear();
    }
}

fn allow_additional_properties(obj: &mut SchemaObject) {
    if let Some(object_validation) = obj.object.as_mut() {
        object_validation.additional_properties = Some(Box::new(Schema::Bool(true)));
    }
}

fn deny_additional_properties(obj: &mut SchemaObject) {
    if let Some(object_validation) = obj.object.as_mut() {
        object_validation.additional_properties = Some(Box::new(Schema::Bool(false)));
    }
}
