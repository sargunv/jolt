//! dprint wasm plugin wrapper for Jolt.

pub mod configuration;
pub mod handler;

pub use handler::JoltDprintPlugin;

#[cfg(all(feature = "wasm", target_arch = "wasm32", target_os = "unknown"))]
use dprint_core::plugins::SyncPluginHandler;

#[cfg(all(feature = "wasm", target_arch = "wasm32", target_os = "unknown"))]
dprint_core::generate_plugin_code!(JoltDprintPlugin, JoltDprintPlugin::new(), FormatOptions);

#[cfg(all(feature = "wasm", target_arch = "wasm32", target_os = "unknown"))]
type FormatOptions = jolt_fmt_core::FormatOptions;
