use jolt_fmt_ir::{Doc, FormatOptions, IndentStyle, RenderOptions, TextWidth};

use crate::rules::program::format_file;

pub(crate) struct KotlinFormatter<'a> {
    options: &'a FormatOptions,
}

impl<'a> KotlinFormatter<'a> {
    pub(crate) fn new(options: &'a FormatOptions) -> Self {
        Self { options }
    }

    pub(crate) fn format_file(&mut self, file: &jolt_kotlin_syntax::KotlinFile<'a>) -> Doc<'a> {
        format_file(file, self)
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
