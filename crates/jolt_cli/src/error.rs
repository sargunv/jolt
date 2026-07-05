use std::{error::Error, fmt};

#[derive(Debug)]
pub(crate) struct CliError {
    message: String,
}

impl CliError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn with_prefix(self, prefix: impl fmt::Display) -> Self {
        Self::new(format!("{prefix}: {}", self.message))
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CliError {}
