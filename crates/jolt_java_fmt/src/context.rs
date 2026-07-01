use jolt_fmt_ir::{Doc, IndentStyle, LineEnding, RenderOptions, TextWidth};
use jolt_java_syntax::CompilationUnit;

use crate::format::JavaFormatOptions;
use crate::rules::program::ProgramRule;

pub(crate) trait FormatRule<N> {
    fn fmt(&self, node: &N, formatter: &mut JavaFormatter<'_>) -> Doc;
}

pub(crate) struct JavaFormatter<'a> {
    options: &'a JavaFormatOptions,
}

impl<'a> JavaFormatter<'a> {
    pub(crate) const fn new(options: &'a JavaFormatOptions) -> Self {
        Self { options }
    }

    pub(crate) fn format_compilation_unit(&mut self, unit: &CompilationUnit) -> Doc {
        ProgramRule.fmt(unit, self)
    }

    pub(crate) fn render_options(&self) -> RenderOptions {
        RenderOptions {
            line_width: TextWidth::from(self.options.line_width),
            indent_width: u16::from(self.options.indent_width),
            indent_style: if self.options.use_tabs {
                IndentStyle::Tab
            } else {
                IndentStyle::Space
            },
            line_ending: LineEnding::Lf,
        }
    }
}
