use jolt_fmt_ir::space;
use std::ops::Range;

use jolt_fmt_ir::{Doc, concat, empty_line, hard_line};
use jolt_java_syntax::{CompilationUnit, CompilationUnitItem, JavaSyntaxKind, PackageDeclaration};

use crate::context::JavaFormatter;
use crate::helpers::blocks::{join_empty_lines, join_hard_lines};
use crate::helpers::comments::{
    LeadingTrivia, comments_from_tokens, format_comment, format_removed_comments,
    format_token_sequence, format_token_with_comments,
};
use crate::helpers::formatter_ignore::{
    formatter_ignore_ranges, formatter_ignore_run_doc, formatter_ignore_runs, token_range_between,
};
use crate::rules::annotations::format_annotation;
use crate::rules::comments::format_comment_only_compilation_unit;
use crate::rules::declarations::{format_method_declaration, format_type_declaration};
use crate::rules::imports::format_imports;
use crate::rules::modules::format_module_declaration;
use crate::rules::names::format_name;
use crate::rules::variables::format_field_declaration;

pub(crate) fn format_compilation_unit<'source>(
    unit: &CompilationUnit<'source>,
    formatter: &mut JavaFormatter<'_>,
) -> Doc<'source> {
    let items = unit.items_with_recovered().collect::<Vec<_>>();
    let contents = if items.is_empty() || items.iter().all(is_recovered_eof_token) {
        format_comment_only_compilation_unit(unit)
    } else {
        let ignored_ranges = formatter_ignore_ranges(
            unit.source_text(),
            unit.text_range().start().get(),
            unit.token_iter(),
        );
        if ignored_ranges.is_empty() {
            return concat([
                format_compilation_unit_item_entries(items, formatter)
                    .unwrap_or_else(jolt_fmt_ir::nil),
                hard_line(),
            ]);
        }
        let item_ranges = items
            .iter()
            .map(recovered_compilation_unit_item_token_range)
            .collect::<Vec<_>>();
        let ignored_runs = formatter_ignore_runs(&ignored_ranges, &item_ranges);
        format_compilation_unit_item_entries_with_ignored(items, &ignored_runs, formatter)
    };

    concat([contents, hard_line()])
}

fn is_recovered_eof_token(
    item: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, CompilationUnitItem<'_>>,
) -> bool {
    matches!(
        item,
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token)
            if token.kind() == JavaSyntaxKind::Eof
    )
}

fn format_compilation_unit_item_entries<'source>(
    items: Vec<
        jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
    >,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let mut sections = Vec::new();
    let mut segment = Vec::new();
    for item in items {
        match item {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(item) => segment.push(item),
            recovered => {
                push_compilation_unit_segment(&mut sections, &mut segment, formatter);
                push_compilation_unit_recovered_section(&mut sections, recovered);
            }
        }
    }
    push_compilation_unit_segment(&mut sections, &mut segment, formatter);

    (!sections.is_empty()).then(|| join_program_sections(sections))
}

fn format_compilation_unit_items<'source>(
    items: Vec<CompilationUnitItem<'source>>,
    formatter: &JavaFormatter<'_>,
) -> Option<Doc<'source>> {
    let mut sections = Vec::new();
    let mut package = None;
    let mut imports = Vec::new();
    let mut module = None;
    let mut declarations = Vec::new();

    for item in items {
        match item {
            CompilationUnitItem::Package(declaration) => package = Some(declaration),
            CompilationUnitItem::Import(declaration) => imports.push(declaration),
            CompilationUnitItem::Module(declaration) => module = Some(declaration),
            CompilationUnitItem::Type(declaration) => {
                declarations.push(format_type_declaration(&declaration, formatter));
            }
            CompilationUnitItem::Field(declaration) => {
                declarations.push(format_field_declaration(&declaration, formatter));
            }
            CompilationUnitItem::Method(declaration) => {
                declarations.push(format_method_declaration(&declaration, formatter));
            }
            CompilationUnitItem::EmptyDeclaration(declaration) => {
                if let Some(comments) =
                    format_removed_comments(comments_from_tokens(declaration.token_iter()))
                {
                    declarations.push(comments);
                }
            }
        }
    }

    if let Some(package) = package {
        sections.push(format_package_declaration(&package, formatter));
    }

    let imports = format_imports(imports);
    if let Some(imports) = imports {
        sections.push(imports);
    }

    if let Some(module) = module {
        sections.push(format_module_declaration(&module));
    }

    if !declarations.is_empty() {
        sections.push(join_empty_lines(declarations));
    }

    (!sections.is_empty()).then(|| join_empty_lines(sections))
}

fn format_compilation_unit_item_entries_with_ignored<'source>(
    items: Vec<
        jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
    >,
    ignored_runs: &[crate::helpers::formatter_ignore::FormatterIgnoreRun<'source>],
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let mut sections = Vec::new();
    let mut segment = Vec::new();
    let mut ignored_index = 0;
    let mut skip_index = 0;

    for (item_index, item) in items.into_iter().enumerate() {
        while ignored_index < ignored_runs.len()
            && ignored_runs[ignored_index].insert_index == item_index
        {
            push_compilation_unit_segment(&mut sections, &mut segment, formatter);
            let run = &ignored_runs[ignored_index];
            sections.push(ProgramSection {
                doc: formatter_ignore_run_doc(run),
                hard_line_after: !run.include_on_marker,
            });
            ignored_index += 1;
        }

        while skip_index < ignored_runs.len() && ignored_runs[skip_index].skip_end <= item_index {
            skip_index += 1;
        }

        if skip_index < ignored_runs.len() && ignored_runs[skip_index].skips(item_index) {
            continue;
        }

        match item {
            jolt_java_syntax::RecoveredSeparatedListEntry::Entry(item) => segment.push(item),
            recovered => {
                push_compilation_unit_segment(&mut sections, &mut segment, formatter);
                push_compilation_unit_recovered_section(&mut sections, recovered);
            }
        }
    }

    push_compilation_unit_segment(&mut sections, &mut segment, formatter);
    while ignored_index < ignored_runs.len() {
        let run = &ignored_runs[ignored_index];
        sections.push(ProgramSection {
            doc: formatter_ignore_run_doc(run),
            hard_line_after: !run.include_on_marker,
        });
        ignored_index += 1;
    }

    join_program_sections(sections)
}

fn push_compilation_unit_segment<'source>(
    sections: &mut Vec<ProgramSection<'source>>,
    segment: &mut Vec<CompilationUnitItem<'source>>,
    formatter: &JavaFormatter<'_>,
) {
    if segment.is_empty() {
        return;
    }
    let items = std::mem::take(segment);
    if let Some(doc) = format_compilation_unit_items(items, formatter) {
        sections.push(ProgramSection {
            doc,
            hard_line_after: false,
        });
    }
}

fn push_compilation_unit_recovered_section<'source>(
    sections: &mut Vec<ProgramSection<'source>>,
    item: jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
) {
    let is_eof = is_recovered_eof_token(&item);
    let Some(doc) = format_recovered_compilation_unit_item(item) else {
        return;
    };
    if is_eof && let Some(previous) = sections.last_mut() {
        previous.hard_line_after = true;
    }
    sections.push(ProgramSection {
        doc,
        hard_line_after: false,
    });
}

fn join_program_sections(sections: Vec<ProgramSection<'_>>) -> Doc<'_> {
    let mut joined = Vec::new();
    let mut previous_hard_line_after = false;
    for section in sections {
        if !joined.is_empty() {
            joined.push(if previous_hard_line_after {
                hard_line()
            } else {
                empty_line()
            });
        }
        joined.push(section.doc);
        previous_hard_line_after = section.hard_line_after;
    }
    concat(joined)
}

struct ProgramSection<'source> {
    doc: Doc<'source>,
    hard_line_after: bool,
}

fn compilation_unit_item_token_range(item: &CompilationUnitItem<'_>) -> Option<Range<usize>> {
    Some(token_range_between(
        &item.first_token()?,
        &item.last_token()?,
    ))
}

fn recovered_compilation_unit_item_token_range(
    item: &jolt_java_syntax::RecoveredSeparatedListEntry<'_, CompilationUnitItem<'_>>,
) -> Option<Range<usize>> {
    match item {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(item) => {
            compilation_unit_item_token_range(item)
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            Some(token_range_between(token, token))
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => Some(token_range_between(
            &error.first_token()?,
            &error.last_token()?,
        )),
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => Some(token_range_between(
            &node.first_token()?,
            &node.last_token()?,
        )),
    }
}

fn format_recovered_compilation_unit_item<'source>(
    item: jolt_java_syntax::RecoveredSeparatedListEntry<'source, CompilationUnitItem<'source>>,
) -> Option<Doc<'source>> {
    match item {
        jolt_java_syntax::RecoveredSeparatedListEntry::Entry(_) => None,
        jolt_java_syntax::RecoveredSeparatedListEntry::Token(token) => {
            if token.kind() == JavaSyntaxKind::Eof {
                Some(join_hard_lines(
                    token
                        .leading_comments()
                        .chain(token.trailing_comments())
                        .map(|comment| format_comment(&comment)),
                ))
            } else {
                Some(format_token_sequence(
                    std::iter::once(token),
                    LeadingTrivia::Preserve,
                ))
            }
        }
        jolt_java_syntax::RecoveredSeparatedListEntry::Error(error) => Some(format_token_sequence(
            error.token_iter(),
            LeadingTrivia::Preserve,
        )),
        jolt_java_syntax::RecoveredSeparatedListEntry::Node(node) => Some(format_token_sequence(
            node.token_iter(),
            LeadingTrivia::Preserve,
        )),
    }
}

fn format_package_declaration<'source>(
    package: &PackageDeclaration<'source>,
    formatter: &JavaFormatter<'_>,
) -> Doc<'source> {
    let annotations = package
        .annotations()
        .map(|annotation| format_annotation(&annotation, formatter))
        .collect::<Vec<_>>();
    let declaration = concat([
        package
            .package_token()
            .map_or_else(jolt_fmt_ir::nil, |token| {
                concat([format_token_with_comments(&token), space()])
            }),
        package
            .name()
            .map_or_else(jolt_fmt_ir::nil, |name| format_name(&name)),
        package
            .semicolon()
            .map_or_else(jolt_fmt_ir::nil, |token| format_token_with_comments(&token)),
    ]);

    if annotations.is_empty() {
        declaration
    } else {
        concat([join_hard_lines(annotations), hard_line(), declaration])
    }
}

#[cfg(test)]
mod tests {
    use jolt_fmt_ir::{
        IndentStyle, RenderControl, RenderOptions, RenderSink, TextWidth, render_to,
    };
    use jolt_java_syntax::parse_compilation_unit;

    use crate::context::JavaFormatter;
    use crate::format::JavaFormatOptions;

    #[test]
    fn recovered_argument_list_preserves_orphan_tokens_and_comments() {
        let formatted = format("class C { void m() { call(first, /* orphan */ , second); } }\n");

        assert!(formatted.contains("call"));
        assert!(formatted.contains("first"));
        assert!(formatted.contains("/* orphan */"));
        assert!(formatted.contains(','));
        assert!(formatted.contains("second"));
    }

    #[test]
    fn recovered_type_annotation_and_parameter_lists_preserve_orphan_tokens() {
        let source = "class C<T, /* type */ , U> { @A(x = 1, /* ann */ , y = 2) void m(int a, /* param */ , int b) {} }\n";
        let formatted = format(source);

        assert!(formatted.contains('T'));
        assert!(formatted.contains("/* type */"));
        assert!(formatted.contains('U'));
        assert!(formatted.contains("/* ann */"));
        assert!(formatted.contains("/* param */"));
        assert!(formatted.contains("int b"));
    }

    #[test]
    fn recovered_block_statement_preserves_unstructured_tokens_and_comments() {
        let formatted = format("class C { void m() { int value = /* keep */ ; } }\n");

        assert!(formatted.contains("int"));
        assert!(formatted.contains("value"));
        assert!(formatted.contains('='));
        assert!(formatted.contains("/* keep */"));
    }

    #[test]
    fn empty_statement_comments_are_preserved() {
        let formatted = format("class C { void m(boolean ready) { while (ready) ; // keep\n} }\n");

        assert!(formatted.contains("while"), "{formatted}");
        assert!(formatted.contains("// keep"), "{formatted}");
    }

    #[test]
    fn top_level_empty_declaration_comments_are_preserved() {
        let formatted = format("class A {}\n; // keep\nclass B {}\n");

        assert!(formatted.contains("class A"), "{formatted}");
        assert!(formatted.contains("// keep"), "{formatted}");
        assert!(formatted.contains("class B"), "{formatted}");
    }

    #[test]
    fn recovered_method_invocation_preserves_structured_receiver_without_name() {
        let formatted = format("class C { void m() { obj.(); } }\n");

        assert!(formatted.contains("obj"), "{formatted}");
        assert!(formatted.contains('.'), "{formatted}");
        assert!(formatted.contains("()"), "{formatted}");
    }

    #[test]
    fn recovered_binary_expression_does_not_recurse_or_drop_operator() {
        let formatted = format("class C { void m() { value + /* rhs */ ; } }\n");

        assert!(formatted.contains("value"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
        assert!(formatted.contains("/* rhs */"), "{formatted}");
    }

    #[test]
    fn recovered_import_and_module_directives_do_not_panic() {
        let formatted = format("import ;\nmodule m { requires ; }\n");

        assert!(formatted.contains("import"));
        assert!(formatted.contains(';'));
        assert!(formatted.contains("module"));
        assert!(formatted.contains("requires"));
    }

    #[test]
    fn recovered_empty_clauses_preserve_keywords_connectives_and_comments() {
        let source = concat!(
            "class Base {}\n",
            "class C extends /* type */ {}\n",
            "class D { void m() throws /* throws */ {} }\n",
            "module m { exports p to /* target */ ; provides S with /* impl */ ; }\n",
        );
        let formatted = format(source);

        assert!(formatted.contains("extends"));
        assert!(formatted.contains("/* type */"));
        assert!(formatted.contains("throws"));
        assert!(formatted.contains("/* throws */"));
        assert!(formatted.contains("to"));
        assert!(formatted.contains("/* target */"));
        assert!(formatted.contains("with"));
        assert!(formatted.contains("/* impl */"));
    }

    #[test]
    fn recovered_member_body_preserves_unowned_tokens_between_structured_members() {
        let formatted = format("class C { int a; /* member */ + ; void m() {} }\n");

        assert!(formatted.contains("int a"), "{formatted}");
        assert!(formatted.contains("/* member */"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
        assert!(formatted.contains("void m()"), "{formatted}");
    }

    #[test]
    fn recovered_member_body_with_formatter_ignore_preserves_unowned_tokens() {
        let source = concat!(
            "class C {\n",
            "  int before;\n",
            "  /* member */ + ;\n",
            "  // @formatter:off\n",
            "  int raw=1+2;\n",
            "  // @formatter:on\n",
            "  int after;\n",
            "}\n",
        );
        let formatted = format(source);

        assert!(formatted.contains("int before"), "{formatted}");
        assert!(formatted.contains("/* member */"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
        assert!(formatted.contains("int raw=1+2;"), "{formatted}");
        assert!(formatted.contains("int after"), "{formatted}");
    }

    #[test]
    fn recovered_resource_and_catch_lists_preserve_unowned_tokens() {
        let source = "class C { void m() { try (a + /* resource */ ; b) {} catch (A | /* catch */ | B e) {} } }\n";
        let formatted = format(source);

        assert!(formatted.contains('a'), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
        assert!(formatted.contains("/* resource */"), "{formatted}");
        assert!(formatted.contains('b'), "{formatted}");
        assert!(formatted.contains('A'), "{formatted}");
        assert!(formatted.contains("/* catch */"), "{formatted}");
        assert!(formatted.contains("B e"), "{formatted}");
    }

    #[test]
    fn recovered_switch_and_module_lists_preserve_unowned_tokens() {
        let source = concat!(
            "class C { void m(Object value) { switch (value) { case A, /* case */ , B -> use(); } } }\n",
            "module m { exports p to a, /* module */ , b; }\n",
        );
        let formatted = format(source);

        assert!(formatted.contains('A'));
        assert!(formatted.contains("/* case */"));
        assert!(formatted.contains('B'));
        assert!(formatted.contains("exports"));
        assert!(formatted.contains("/* module */"));
        assert!(formatted.contains('b'));
    }

    #[test]
    fn recovered_switch_block_preserves_top_level_unowned_tokens() {
        let formatted = format(
            "class C { void m(int value) { switch (value) { /* block */ + ; case 1 -> use(); } } }\n",
        );

        assert!(formatted.contains("/* block */"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
        assert!(formatted.contains("case"), "{formatted}");
        assert!(formatted.contains("use"), "{formatted}");
    }

    #[test]
    fn recovered_switch_rule_with_empty_body_preserves_semicolon() {
        let formatted = format(
            "class C { void m(int value) { switch (value) { case 1 -> /* empty */ ; } } }\n",
        );

        assert!(formatted.contains("case"), "{formatted}");
        assert!(formatted.contains("->"), "{formatted}");
        assert!(formatted.contains("/* empty */"), "{formatted}");
        assert!(formatted.contains(';'), "{formatted}");
    }

    #[test]
    fn recovered_block_with_formatter_ignore_preserves_unowned_tokens() {
        let source = concat!(
            "class C { void m() {\n",
            "  int before;\n",
            "  // @formatter:off\n",
            "  int raw=1+2;\n",
            "  // @formatter:on\n",
            "  /* block */ + ;\n",
            "  int after;\n",
            "} }\n",
        );
        let formatted = format(source);

        assert!(formatted.contains("int before"), "{formatted}");
        assert!(formatted.contains("int raw=1+2;"), "{formatted}");
        assert!(formatted.contains("/* block */"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
    }

    #[test]
    fn recovered_switch_statement_group_preserves_unowned_tokens() {
        let formatted = format(
            "class C { void m(int value) { switch (value) { case 1: /* group */ + ; use(); } } }\n",
        );

        assert!(formatted.contains("case"), "{formatted}");
        assert!(formatted.contains("/* group */"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
        assert!(formatted.contains("use"), "{formatted}");
    }

    #[test]
    fn recovered_constructor_body_preserves_unowned_tokens() {
        let formatted = format("class C { C() { this(); /* body */ + ; int value = 1; } }\n");

        assert!(formatted.contains("this()"), "{formatted}");
        assert!(formatted.contains("/* body */"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
        assert!(formatted.contains("int"), "{formatted}");
        assert!(formatted.contains("value"), "{formatted}");
    }

    #[test]
    fn recovered_qualifier_dots_are_preserved_without_qualifiers() {
        let formatted =
            format("class C { C() { <T>.this(); } void m() { .new Nested(); .this; .super; } }\n");

        assert!(formatted.contains('.'), "{formatted}");
        assert!(formatted.contains("this"), "{formatted}");
        assert!(formatted.contains("super"), "{formatted}");
        assert!(formatted.contains(".new"), "{formatted}");
    }

    #[test]
    fn recovered_compilation_unit_preserves_top_level_unowned_tokens() {
        let source = "/* top */ + ; class C {}\n// JOLT-TRIVIA:file-tail\n";
        let formatted = format(source);

        assert!(formatted.contains("/* top */"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
        assert!(formatted.contains("class C"), "{formatted}");
        assert!(
            formatted.contains("// JOLT-TRIVIA:file-tail"),
            "{formatted}"
        );
        assert!(
            !formatted.contains("}\n\n// JOLT-TRIVIA:file-tail"),
            "{formatted}"
        );
        assert_eq!(format(&formatted), formatted);
    }

    #[test]
    fn catch_parameter_modifiers_use_structured_modifier_entries() {
        let formatted = format("class C { void m() { try {} catch (final /* mod */ E e) {} } }\n");

        assert!(formatted.contains("final"), "{formatted}");
        assert!(formatted.contains("/* mod */"), "{formatted}");
        assert!(formatted.contains("E e"), "{formatted}");
    }

    #[test]
    fn recovered_enum_pattern_and_for_lists_preserve_unowned_tokens() {
        let source = concat!(
            "enum E { A, /* enum */ , B }\n",
            "record Point(int x, int y) {}\n",
            "class C { void m(Object value) { ",
            "if (value instanceof Point(int x, /* pattern */ , int y)) {} ",
            "for (i = 0, /* init */ , j = 0; ; i++, /* update */ , j++) {} ",
            "} }\n",
        );
        let formatted = format(source);

        assert!(formatted.contains("/* enum */"), "{formatted}");
        assert!(formatted.contains("/* pattern */"), "{formatted}");
        assert!(formatted.contains("/* init */"), "{formatted}");
        assert!(formatted.contains("/* update */"), "{formatted}");
    }

    #[test]
    fn recovered_module_body_preserves_unowned_tokens_between_directives() {
        let formatted = format("module m { requires a; /* module */ + ; exports p; }\n");

        assert!(formatted.contains("requires a"), "{formatted}");
        assert!(formatted.contains("/* module */"), "{formatted}");
        assert!(formatted.contains('+'), "{formatted}");
        assert!(formatted.contains("exports p"), "{formatted}");
        assert_eq!(format(&formatted), formatted);
    }

    #[test]
    fn recovered_header_clauses_preserve_orphan_separators_and_comments() {
        let source = concat!(
            "sealed class C extends Base, /* extends */ , Other ",
            "implements A, /* implements */ , B permits C, /* permits */ , D { ",
            "void m() throws E, /* throws */ , F {} }\n",
        );
        let formatted = format(source);

        assert!(formatted.contains("extends"), "{formatted}");
        assert!(formatted.contains("/* extends */"), "{formatted}");
        assert!(formatted.contains("implements"), "{formatted}");
        assert!(formatted.contains("/* implements */"), "{formatted}");
        assert!(formatted.contains("permits"), "{formatted}");
        assert!(formatted.contains("/* permits */"), "{formatted}");
        assert!(formatted.contains("throws"), "{formatted}");
        assert!(formatted.contains("/* throws */"), "{formatted}");
    }

    #[test]
    fn recovered_wildcard_bounds_and_type_argument_annotations_are_structural() {
        let formatted =
            format("class C { List<? extends /* bound */> a; List<@A /* arg */> b; }\n");

        assert!(formatted.contains("? extends"), "{formatted}");
        assert!(formatted.contains("/* bound */"), "{formatted}");
        assert!(formatted.contains("@A"), "{formatted}");
        assert!(formatted.contains("/* arg */"), "{formatted}");
    }

    fn format(source: &str) -> String {
        let parse = parse_compilation_unit(source);
        let unit = parse.syntax().expect("test input should produce a tree");
        let options = JavaFormatOptions::default();
        let mut formatter = JavaFormatter::new(&options);
        let doc = formatter.format_compilation_unit(&unit);
        let mut sink = StringDocSink::default();
        render_to(
            &doc,
            RenderOptions {
                line_width: TextWidth::from(80),
                indent_width: 2,
                indent_style: IndentStyle::Space,
            },
            &mut sink,
        )
        .expect("test doc should render");
        sink.output
    }

    #[derive(Default)]
    struct StringDocSink {
        output: String,
    }

    impl RenderSink for &mut StringDocSink {
        fn write_str(&mut self, text: &str) -> RenderControl {
            self.output.push_str(text);
            RenderControl::Continue
        }
    }
}
