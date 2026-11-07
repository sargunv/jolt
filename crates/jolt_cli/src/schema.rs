//! JSON schemas for `jolt.toml` and the dprint plugin config, both derived from
//! the config types.

use jolt_config::FormatConfig;
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
            .map(|(name, schema)| (camel_case(&name), schema))
            .collect();
    }
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "dprint jolt plugin configuration",
        "type": "object",
        "properties": { "jolt": options },
    })
}

fn camel_case(kebab: &str) -> String {
    let mut words = kebab.split('-');
    let mut camel = words.next().unwrap_or_default().to_owned();
    for word in words {
        let mut chars = word.chars();
        camel.extend(chars.next().map(|first| first.to_ascii_uppercase()));
        camel.push_str(chars.as_str());
    }
    camel
}
