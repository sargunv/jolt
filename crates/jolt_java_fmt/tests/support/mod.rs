use std::convert::Infallible;

use jolt_diagnostics::Diagnostic;
use jolt_fmt_ir::{RenderControl, RenderSink};
use jolt_java_fmt::{JavaFormatOptions, JavaFormatSinkResult, format_source_to_sink};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestFormatResult {
    pub formatted_source: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Default)]
struct StringSink {
    text: String,
}

impl RenderSink for StringSink {
    type Error = Infallible;

    fn write_str(&mut self, text: &str) -> Result<RenderControl, Self::Error> {
        self.text.push_str(text);
        Ok(RenderControl::Continue)
    }
}

pub fn format_source(source: &str, options: &JavaFormatOptions) -> TestFormatResult {
    let mut sink = StringSink::default();
    let result = format_source_to_sink(source, options, &mut sink);
    match result {
        JavaFormatSinkResult::Complete { diagnostics }
        | JavaFormatSinkResult::Halted { diagnostics } => TestFormatResult {
            formatted_source: Some(sink.text),
            diagnostics,
        },
        JavaFormatSinkResult::Blocked { diagnostics } => TestFormatResult {
            formatted_source: None,
            diagnostics,
        },
        JavaFormatSinkResult::SinkError { error, .. } => match error {},
    }
}
