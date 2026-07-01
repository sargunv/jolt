use super::*;

fn test_options(width: u32) -> RenderOptions {
    RenderOptions {
        line_width: TextWidth::new(width),
        indent_width: 2,
        indent_style: IndentStyle::Space,
        line_ending: LineEnding::Lf,
    }
}

fn render_text(doc: &Doc, width: u32) -> String {
    render(doc, test_options(width))
        .expect("document should render")
        .text
}

#[test]
fn text_and_concat_render() {
    let doc = concat([text("class"), text(" "), text("Main")]);
    assert_eq!(render_text(&doc, 80), "class Main");
}

#[test]
fn text_rejects_line_terminators() {
    let err = render(&text("a\nb"), test_options(80)).expect_err("invalid text");
    assert_eq!(err, RenderError::InvalidText { context: "Text" });
}

#[test]
fn group_fits_on_one_line() {
    let doc = group(concat([text("a"), line(), text("b")]));
    assert_eq!(render_text(&doc, 80), "a b");
}

#[test]
fn group_expands_when_width_is_exceeded() {
    let doc = group(concat([text("long"), line(), text("tail")]));
    assert_eq!(render_text(&doc, 6), "long\ntail");
}

#[test]
fn nested_groups_remeasure_after_hard_lines() {
    let doc = group(concat([
        text("a"),
        hard_line(),
        group(concat([text("b"), line(), text("c")])),
    ]));
    assert_eq!(render_text(&doc, 80), "a\nb c");
}

#[test]
fn line_modes_have_distinct_flat_and_expanded_behavior() {
    let flat = group(concat([
        text("a"),
        soft_line(),
        text("b"),
        line(),
        text("c"),
    ]));
    assert_eq!(render_text(&flat, 80), "ab c");

    let expanded = force_group(concat([
        text("a"),
        soft_line(),
        text("b"),
        line(),
        text("c"),
    ]));
    assert_eq!(render_text(&expanded, 80), "a\nb\nc");

    assert_eq!(
        render_text(&concat([text("a"), hard_line(), text("b")]), 80),
        "a\nb"
    );
    assert_eq!(
        render_text(&concat([text("a"), empty_line(), text("b")]), 80),
        "a\n\nb"
    );
}

#[test]
fn indentation_after_nested_line_breaks() {
    let doc = group(concat([
        text("{"),
        indent(concat([line(), text("x;")])),
        line(),
        text("}"),
    ]));
    assert_eq!(render_text(&doc, 4), "{\n  x;\n}");
}

#[test]
fn break_only_indent_delta() {
    let doc = force_group(concat([text("a"), break_(FlatLine::Space, 2), text("b")]));
    assert_eq!(render_text(&doc, 80), "a\n    b");
}

#[test]
fn alignment_spaces_apply_after_breaks() {
    let doc = force_group(align(3, concat([text("a"), line(), text("b")])));
    assert_eq!(render_text(&doc, 80), "a\n   b");
}

#[test]
fn dedent_removes_one_indent_level_after_breaks() {
    let doc = force_group(indent(concat([
        text("a"),
        dedent(concat([line(), text("b"), line(), text("c")])),
    ])));
    assert_eq!(render_text(&doc, 80), "a\nb\nc");
}

#[test]
fn if_break_uses_current_group() {
    let doc = group(concat([
        text("["),
        line(),
        text("x"),
        if_break(text(","), nil()),
        line(),
        text("]"),
    ]));
    assert_eq!(render_text(&doc, 80), "[ x ]");
    assert_eq!(render_text(&doc, 3), "[\nx,\n]");
}

#[test]
fn if_break_uses_labelled_group() {
    let id = GroupId(7);
    let doc = concat([
        group_id(id, concat([text("x"), line(), text("y")])),
        if_group_breaks(id, text(" broke"), text(" flat")),
    ]);
    assert_eq!(render_text(&doc, 80), "x y flat");
    assert_eq!(render_text(&doc, 2), "x\ny broke");
}

#[test]
fn indent_if_break_uses_labelled_group() {
    let id = GroupId(1);
    let doc = group_id(
        id,
        concat([
            text("call("),
            indent_if_break(id, concat([line(), text("argument")])),
            line(),
            text(")"),
        ]),
    );
    assert_eq!(render_text(&doc, 80), "call( argument )");
    assert_eq!(render_text(&doc, 8), "call(\n  argument\n)");
}

#[test]
fn fill_packs_independently() {
    let doc = fill(
        [
            fill_entry(text("alpha"), line()),
            fill_entry(text("beta"), line()),
        ],
        text("gamma"),
    );
    assert_eq!(render_text(&doc, 11), "alpha beta\ngamma");
}

#[test]
fn unselected_if_break_branch_does_not_expand_parent_group() {
    let doc = group(concat([
        text("["),
        if_break(concat([hard_line(), text("broken")]), text("flat")),
        text("]"),
    ]));
    let rendered = render(&doc, test_options(80)).expect("document should render");

    assert_eq!(rendered.text, "[flat]");
    assert_eq!(rendered.stats.expanded_group_count, 0);
}

#[test]
fn line_suffix_flushes_before_newline_and_boundary() {
    let doc = concat([
        text("x"),
        line_suffix(text(" // trailing")),
        hard_line(),
        text("y"),
    ]);
    assert_eq!(render_text(&doc, 80), "x // trailing\ny");

    let boundary = concat([
        text("x"),
        line_suffix(text(" // bounded")),
        line_suffix_boundary(),
        text(";"),
    ]);
    assert_eq!(render_text(&boundary, 80), "x // bounded;");
}

#[test]
fn nested_line_suffixes_flush_on_the_same_boundary() {
    let doc = concat([
        line_suffix(concat([text(" // outer"), line_suffix(text(" // inner"))])),
        hard_line(),
        text("next"),
    ]);

    assert_eq!(render_text(&doc, 80), " // outer // inner\nnext");
}

#[test]
fn line_suffix_rejects_newline_producing_content() {
    let err = render(
        &line_suffix(concat([text(" // outer"), hard_line(), text("after")])),
        test_options(80),
    )
    .expect_err("line suffixes must not create line breaks");

    assert_eq!(
        err,
        RenderError::InvalidLineSuffix {
            reason: "hard line"
        }
    );
}

#[test]
fn literal_text_preserves_newlines_and_updates_column() {
    let rendered = render_text(&concat([literal_text("a\nbc"), text("d")]), 80);
    assert_eq!(rendered, "a\nbcd");
}

#[test]
fn literal_text_updates_max_column_for_intermediate_lines() {
    let rendered = render(
        &concat([text("prefix"), literal_text("wide\nx")]),
        test_options(80),
    )
    .expect("document should render");

    assert_eq!(rendered.text, "prefixwide\nx");
    assert_eq!(rendered.stats.max_column, TextWidth::new(10));
}

#[test]
fn literal_text_resets_base_column_after_each_embedded_newline() {
    let rendered = render(
        &concat([text("0123456789"), literal_text("a\nbbbbbbbb\nc")]),
        test_options(80),
    )
    .expect("document should render");

    assert_eq!(rendered.text, "0123456789a\nbbbbbbbb\nc");
    assert_eq!(rendered.stats.max_column, TextWidth::new(11));
    assert_eq!(rendered.stats.line_count, 3);
}

#[test]
fn explicit_literal_width_updates_final_column() {
    let doc = concat([
        literal_text_with_width("a\nbc", TextWidth::new(20)),
        group(concat([line(), text("tail")])),
    ]);
    assert_eq!(render_text(&doc, 22), "a\nbc\ntail");
}

#[test]
fn explicit_literal_line_widths_update_intermediate_columns() {
    let rendered = render(
        &literal_text_with_line_widths("prefix\nsuffix", [TextWidth::new(30), TextWidth::new(6)])
            .expect("line widths match"),
        test_options(80),
    )
    .expect("document should render");

    assert_eq!(rendered.text, "prefix\nsuffix");
    assert_eq!(rendered.stats.max_column, TextWidth::new(30));
}

#[test]
fn explicit_literal_line_width_count_must_match_literal_lines() {
    let err = literal_text_with_line_widths("prefix\nsuffix", [TextWidth::new(30)])
        .expect_err("line width count should be validated");

    assert_eq!(
        err,
        RenderError::InvalidLiteralWidths {
            expected: 2,
            actual: 1,
        }
    );
}

#[test]
fn break_parent_propagates_expansion() {
    let doc = group(concat([text("a"), break_parent(), line(), text("b")]));
    assert_eq!(render_text(&doc, 80), "a\nb");
}

#[test]
fn non_propagating_hard_line_does_not_mark_group_broken() {
    let doc = group(concat([
        text("a"),
        hard_line_without_break_parent(),
        if_break(text("broken"), text("flat")),
    ]));
    let rendered = render(&doc, test_options(80)).expect("document should render");

    assert_eq!(rendered.text, "a\nflat");
    assert_eq!(rendered.stats.expanded_group_count, 0);
}

#[test]
fn explicit_text_width_affects_fitting() {
    let doc = group(concat([
        text_with_width("wide", TextWidth::new(20)),
        line(),
        text("tail"),
    ]));
    assert_eq!(render_text(&doc, 10), "wide\ntail");
}

#[test]
fn default_text_width_uses_unicode_display_width() {
    let doc = group(concat([text("界界"), line(), text("x")]));
    assert_eq!(render_text(&doc, 6), "界界 x");
    assert_eq!(render_text(&doc, 5), "界界\nx");
}

#[test]
fn explicit_java_width_can_override_unicode_width() {
    let unicode_width_doc = group(concat([text("e\u{301}"), line(), text("x")]));
    let java_width_doc = group(concat([
        text_with_width("e\u{301}", TextWidth::new(2)),
        line(),
        text("x"),
    ]));

    assert_eq!(render_text(&unicode_width_doc, 3), "e\u{301} x");
    assert_eq!(render_text(&java_width_doc, 3), "e\u{301}\nx");
}

#[test]
fn flat_line_text_rejects_line_terminators() {
    let err = render(
        &group(concat([text("a"), break_(flat_text(" \n"), 0), text("b")])),
        test_options(80),
    )
    .expect_err("invalid flat text");
    assert_eq!(
        err,
        RenderError::InvalidText {
            context: "FlatLine::Text"
        }
    );
}

#[test]
fn indent_if_break_rejects_unknown_group_id() {
    let err = render(&indent_if_break(GroupId(99), text("x")), test_options(80))
        .expect_err("unknown group");
    assert_eq!(err, RenderError::UnknownGroupId(GroupId(99)));
}

#[test]
fn pending_line_suffix_width_affects_group_fitting() {
    let doc = concat([
        line_suffix(text("xxx")),
        group(concat([text("ab"), line(), text("cd")])),
    ]);

    assert_eq!(render_text(&doc, 7), "abxxx\ncd");
}

#[test]
fn pending_line_suffix_if_break_uses_actual_group_state_for_fitting() {
    let id = GroupId(10);
    let doc = concat([
        force_group_id(id, concat([text("aa"), line(), text("bb")])),
        line_suffix(if_group_breaks(id, text("xxxxx"), text("x"))),
        group(concat([text("c"), line(), text("d")])),
    ]);

    assert_eq!(render_text(&doc, 8), "aa\nbbcxxxxx\nd");
}

#[test]
fn render_stats_report_lines_groups_and_suffixes() {
    let rendered = render(
        &concat([
            group(concat([text("a"), line(), text("b")])),
            line_suffix(text(" // suffix")),
            group(concat([text("c"), line(), text("d")])),
        ]),
        test_options(2),
    )
    .expect("document should render");

    assert_eq!(rendered.text, "a\nbc // suffix\nd");
    assert_eq!(
        rendered.stats,
        RenderStats {
            line_count: 3,
            max_column: TextWidth::new(12),
            group_count: 2,
            expanded_group_count: 2,
            line_suffix_count: 1,
        }
    );
}

mod java_shapes {
    use super::*;

    #[test]
    fn method_invocation_arguments() {
        let doc = group(concat([
            text("call("),
            indent(concat([
                soft_line(),
                join(concat([text(","), line()]), [text("first"), text("second")]),
            ])),
            soft_line(),
            text(")"),
        ]));
        assert_eq!(render_text(&doc, 80), "call(first, second)");
        assert_eq!(render_text(&doc, 14), "call(\n  first,\n  second\n)");
    }

    #[test]
    fn chained_method_calls() {
        let doc = force_group(indent(concat([
            text("builder"),
            break_(FlatLine::Empty, 0),
            text(".setName(name)"),
            break_(FlatLine::Empty, 0),
            text(".build()"),
        ])));
        assert_eq!(
            render_text(&doc, 120),
            "builder\n  .setName(name)\n  .build()"
        );
    }

    #[test]
    fn nested_chain_inside_call_arguments() {
        let arg = group(concat([
            text("builder"),
            break_(FlatLine::Empty, 1),
            text(".set(x)"),
            break_(FlatLine::Empty, 1),
            text(".build()"),
        ]));
        let doc = group(concat([
            text("call("),
            indent(concat([soft_line(), arg])),
            soft_line(),
            text(")"),
        ]));
        assert_eq!(render_text(&doc, 80), "call(builder.set(x).build())");
        assert_eq!(
            render_text(&doc, 18),
            "call(\n  builder\n    .set(x)\n    .build()\n)"
        );
    }

    #[test]
    fn lambda_argument_followed_by_chained_calls() {
        let lambda = force_group(concat([
            text("x -> {"),
            indent(concat([line(), text("return x;")])),
            line(),
            text("}"),
        ]));
        let doc = concat([text("stream.map("), lambda, text(")"), text(".toList()")]);
        assert_eq!(
            render_text(&doc, 30),
            "stream.map(x -> {\n  return x;\n}).toList()"
        );
    }

    #[test]
    fn class_body_with_blank_lines() {
        let doc = concat([
            text("class A {"),
            indent(concat([
                hard_line(),
                text("int x;"),
                empty_line(),
                text("void f() {}"),
            ])),
            hard_line(),
            text("}"),
        ]);
        assert_eq!(
            render_text(&doc, 80),
            "class A {\n  int x;\n\n  void f() {}\n}"
        );
    }

    #[test]
    fn trailing_line_comments() {
        let doc = concat([
            text("int x = 1;"),
            line_suffix(text(" // value")),
            hard_line(),
            text("int y = 2;"),
        ]);
        assert_eq!(render_text(&doc, 80), "int x = 1; // value\nint y = 2;");
    }

    #[test]
    fn block_comment_before_declaration() {
        let doc = concat([
            literal_text("/*\n * comment\n */"),
            hard_line(),
            text("int x;"),
        ]);
        assert_eq!(render_text(&doc, 80), "/*\n * comment\n */\nint x;");
    }

    #[test]
    fn annotation_argument_list() {
        let doc = group(concat([
            text("@Anno("),
            indent(concat([
                soft_line(),
                join(
                    concat([text(","), line()]),
                    [text("name = \"x\""), text("flag = true")],
                ),
            ])),
            soft_line(),
            text(")"),
        ]));
        assert_eq!(render_text(&doc, 80), "@Anno(name = \"x\", flag = true)");
        assert_eq!(
            render_text(&doc, 20),
            "@Anno(\n  name = \"x\",\n  flag = true\n)"
        );
    }

    #[test]
    fn lambda_body() {
        let doc = force_group(concat([
            text("() -> {"),
            indent(concat([line(), text("run();")])),
            line(),
            text("}"),
        ]));
        assert_eq!(render_text(&doc, 80), "() -> {\n  run();\n}");
    }

    #[test]
    fn text_block_literal() {
        let doc = concat([
            text("String s = "),
            literal_text("\"\"\"\n  text\n  \"\"\""),
            text(";"),
        ]);
        assert_eq!(
            render_text(&doc, 80),
            "String s = \"\"\"\n  text\n  \"\"\";"
        );
    }
}
