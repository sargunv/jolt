//! JSON schemas for `jolt.toml` and the dprint plugin config, both derived from
//! the config types.

use jolt_config::{FormatConfig, dprint_key};
use schemars::schema_for;
use serde_json::{Value, json};

use crate::config::ConfigFile;

pub(crate) fn render(dprint: bool) -> String {
    let schema = if dprint {
        dprint_schema()
    } else {
        json!(schema_for!(ConfigFile))
    };
    serde_json::to_string_pretty(&schema).expect("a JSON value serializes")
}

/// The `jolt` key of `dprint.json`, whose options are camelCase.
fn dprint_schema() -> Value {
    let mut options = json!(schema_for!(FormatConfig));
    let object = options
        .as_object_mut()
        .expect("a struct schema is an object");
    object.remove("$schema");
    object.remove("title");
    if let Some(Value::Object(properties)) = object.get_mut("properties") {
        *properties = std::mem::take(properties)
            .into_iter()
            .map(|(name, schema)| (dprint_key(&name), schema))
            .collect();
        // Keys dprint itself reads from every plugin's object.
        properties.insert(
            "associations".to_owned(),
            json!({
                "description": "Glob patterns of the files this plugin formats (read by dprint).",
                "anyOf": [
                    { "type": "string" },
                    { "type": "array", "items": { "type": "string" } },
                ],
            }),
        );
        properties.insert(
            "locked".to_owned(),
            json!({
                "description": "Read by dprint itself; see its configuration documentation.",
                "type": "boolean",
            }),
        );
    }
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "dprint jolt plugin configuration",
        "type": "object",
        "properties": { "jolt": options },
    })
}
