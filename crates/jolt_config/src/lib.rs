//! The format options as users write them. One type backs `jolt.toml`'s
//! `[format]` table, the `jolt format` flags, the dprint plugin config, and
//! both JSON schemas.

use std::num::{NonZeroU8, NonZeroU16};

use jolt_formatter::FormatOptions;
use serde::{Deserialize, Serialize};

/// Jolt formatting options. An unset option falls through to a less specific
/// layer, and finally to its default.
// Names are kebab-case in `jolt.toml` and on the command line, and camelCase
// in the dprint config (`dprint_key`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FormatConfig {
    /// Preferred maximum line width.
    #[cfg_attr(feature = "clap", arg(long))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_width: Option<NonZeroU16>,

    /// Spaces per indentation level when not using tabs.
    #[cfg_attr(feature = "clap", arg(long))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indent_width: Option<NonZeroU8>,

    /// Indent with tabs instead of spaces.
    #[cfg_attr(
        feature = "clap",
        arg(
            long,
            num_args = 0..=1,
            require_equals = true,
            default_missing_value = "true"
        )
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_tabs: Option<bool>,
}

impl FormatConfig {
    /// Layers `self` over `base`: options `self` leaves unset come from `base`.
    #[must_use]
    pub fn or(self, base: Self) -> Self {
        Self {
            line_width: self.line_width.or(base.line_width),
            indent_width: self.indent_width.or(base.indent_width),
            use_tabs: self.use_tabs.or(base.use_tabs),
        }
    }

    /// The options to format with: unset options take their defaults.
    #[must_use]
    pub fn resolve(self) -> FormatOptions {
        let defaults = FormatOptions::default();
        FormatOptions {
            line_width: self.line_width.map_or(defaults.line_width, NonZeroU16::get),
            indent_width: self
                .indent_width
                .map_or(defaults.indent_width, NonZeroU8::get),
            use_tabs: self.use_tabs.unwrap_or(defaults.use_tabs),
        }
    }
}

impl From<FormatOptions> for FormatConfig {
    fn from(options: FormatOptions) -> Self {
        Self {
            line_width: NonZeroU16::new(options.line_width),
            indent_width: NonZeroU8::new(options.indent_width),
            use_tabs: Some(options.use_tabs),
        }
    }
}

/// The dprint config key for the option `name`: dprint plugins take camelCase
/// keys (`lineWidth`) where `jolt.toml` and the flags take kebab-case
/// (`line-width`).
#[must_use]
pub fn dprint_key(name: &str) -> String {
    let mut words = name.split('-');
    let mut key = words.next().unwrap_or_default().to_owned();
    for word in words {
        let mut chars = word.chars();
        key.extend(chars.next().map(|first| first.to_ascii_uppercase()));
        key.push_str(chars.as_str());
    }
    key
}

/// The option name for the dprint config key `key`, the inverse of
/// [`dprint_key`], or `None` when `key` is not the dprint form of any name.
#[must_use]
pub fn option_name(key: &str) -> Option<String> {
    let mut name = String::with_capacity(key.len() + 2);
    for ch in key.chars() {
        if ch.is_ascii_uppercase() {
            name.push('-');
        }
        name.push(ch.to_ascii_lowercase());
    }
    (!name.starts_with('-') && dprint_key(&name) == key).then_some(name)
}
