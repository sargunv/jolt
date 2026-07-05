//! Emits the dprint plugin configuration JSON schema to stdout.

use schemars::schema::{Schema, SchemaObject};
use schemars::schema_for;
use serde::Serialize;

/// Outer schema shape for dprint's `dprint.jsonc`: plugin options nested under
/// the `jolt` config key, with all other dprint global keys permitted.
#[derive(Serialize, schemars::JsonSchema)]
#[schemars(title = "dprint jolt plugin configuration")]
pub(crate) struct DprintJoltConfig {
    /// Jolt-specific options; dprint global options (`lineWidth`, etc.) are
    /// applied as defaults when these are absent.
    pub(crate) jolt: jolt_formatter::FormatOptions,
}

fn main() {
    let mut schema = schema_for!(DprintJoltConfig);
    clear_required(&mut schema.schema);
    for def in schema.definitions.values_mut() {
        if let Schema::Object(obj) = def {
            clear_required(obj);
        }
    }
    let json = serde_json::to_string_pretty(&schema).expect("schema should serialize");
    println!("{json}");
}

/// Clears the `required` list so users can omit any field and rely on dprint
/// globals or built-in defaults, matching the runtime behavior in
/// `jolt_fmt_dprint::configuration::resolve_config`.
fn clear_required(obj: &mut SchemaObject) {
    if let Some(object_validation) = obj.object.as_mut() {
        object_validation.required.clear();
    }
}
