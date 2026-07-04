use jolt_fmt_ir::{Doc, IndentStyle, RenderOptions, TextWidth};

use crate::format::JavaFormatOptions;
use crate::rules::program::format_compilation_unit;

pub(crate) struct JavaFormatter<'a> {
    options: &'a JavaFormatOptions,
}

impl<'a> JavaFormatter<'a> {
    pub(crate) fn new(options: &'a JavaFormatOptions) -> Self {
        Self { options }
    }

    pub(crate) fn format_compilation_unit(
        &mut self,
        unit: &jolt_java_syntax::CompilationUnit<'a>,
    ) -> Doc<'a> {
        format_compilation_unit(unit, self)
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
        }
    }
}
