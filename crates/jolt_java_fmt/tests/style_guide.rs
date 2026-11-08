//! Checks the Java examples in `docs/guides/style.md`: every ` ```java ` block
//! formats to itself, and a block titled `[Input]` formats to the `[Output]`
//! block after it.
//!
//! A block that parses as the body of a method, such as a few statements, is
//! formatted as one, with the line width widened by the method's indentation
//! so it lays out as it would at column 0. Any other block is formatted as a
//! file.

use jolt_fmt::FormatOptions;
use jolt_test_support::{read_to_string, workspace_root};

mod common;

struct Block {
    line: usize,
    title: Option<String>,
    text: String,
}

#[test]
fn style_guide_examples_are_formatted() {
    let path = workspace_root(env!("CARGO_MANIFEST_DIR")).join("docs/guides/style.md");
    let blocks = java_blocks(&read_to_string(&path));
    assert!(!blocks.is_empty(), "no Java examples in {}", path.display());
    let mut failures = Vec::new();
    let mut blocks = blocks.iter().peekable();
    while let Some(block) = blocks.next() {
        let expected = if block.title.as_deref() == Some("Input") {
            let Some(output) = blocks.next_if(|next| next.title.as_deref() == Some("Output"))
            else {
                failures.push(format!(
                    "line {}: [Input] without an [Output] after it",
                    block.line
                ));
                continue;
            };
            &output.text
        } else {
            &block.text
        };
        match format_example(&block.text) {
            Ok(actual) if actual == *expected => {}
            Ok(actual) => failures.push(format!(
                "line {}: expected\n{expected}\nformatted\n{actual}",
                block.line
            )),
            Err(error) => failures.push(format!("line {}: {error}", block.line)),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The ` ```java ` fences, with the title in `[…]` after the language.
fn java_blocks(markdown: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut lines = markdown.lines().enumerate();
    while let Some((index, line)) = lines.next() {
        let Some(info) = line.strip_prefix("```java") else {
            continue;
        };
        let title = info
            .trim()
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
            .map(str::to_owned);
        let mut text = String::new();
        for (_, line) in lines.by_ref() {
            if line == "```" {
                break;
            }
            text.push_str(line);
            text.push('\n');
        }
        blocks.push(Block {
            line: index + 1,
            title,
            text,
        });
    }
    blocks
}

fn format_example(text: &str) -> Result<String, String> {
    let options = FormatOptions::default();
    let indent = " ".repeat(usize::from(options.indent_width));
    let mut wrapped = String::from("void example() {\n");
    for line in text.lines() {
        if !line.is_empty() {
            wrapped.push_str(&indent);
        }
        wrapped.push_str(line);
        wrapped.push('\n');
    }
    wrapped.push_str("}\n");
    if !jolt_java_syntax::parse(&wrapped).diagnostics().is_empty() {
        if !jolt_java_syntax::parse(text).diagnostics().is_empty() {
            return Err("syntax error as a method body and as a file".into());
        }
        return Ok(common::format(text, options));
    }
    let widened = FormatOptions {
        line_width: options.line_width + u16::from(options.indent_width),
        ..options
    };
    let formatted = common::format(&wrapped, widened);
    let body = formatted
        .strip_prefix("void example() {\n")
        .and_then(|rest| rest.strip_suffix("}\n"))
        .ok_or_else(|| format!("unexpected wrapper output:\n{formatted}"))?;
    Ok(body
        .lines()
        .map(|line| line.strip_prefix(indent.as_str()).unwrap_or(line))
        .fold(String::new(), |mut out, line| {
            out.push_str(line);
            out.push('\n');
            out
        }))
}
