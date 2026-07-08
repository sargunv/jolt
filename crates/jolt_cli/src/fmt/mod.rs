mod args;
pub(crate) mod config;
mod discover;
mod run;

pub(crate) use args::{Args, CliFormatOptions};
pub(crate) use discover::detect_language;
pub(crate) use run::run;
