//! Doc comments whose reading javac makes slow or endless: the formatter
//! finishes each in time linear in its size, printing it as written when the
//! reading can't be bounded. Each input takes well under a second in a debug
//! build, and minutes (or forever) at quadratic cost.

use std::sync::mpsc;
use std::time::Duration;

use jolt_fmt::FormatOptions;

mod common;

/// How long a test input may take, far more than linear work needs.
const LIMIT: Duration = Duration::from_secs(20);

/// Formats a class whose method `m` has the doc comment `comment` (with its
/// indentation and line breaks), failing when that takes longer than
/// [`LIMIT`].
fn format_in_time(comment: &str) -> String {
    let source = format!("class C {{\n{comment}\n    void m() {{}}\n}}\n");
    let (send, receive) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = send.send(common::format(&source, FormatOptions::default()));
    });
    receive
        .recv_timeout(LIMIT)
        .expect("formatting finishes in time")
}

/// javac's Markdown scanner never ends on a last line of list markers with
/// a space after them (it pushes a list item without reading a character).
#[test]
fn list_markers_that_end_a_comment() {
    for markers in ["+ + ", "* * ", "1. 1. ", "- - - "] {
        let formatted = format_in_time(&format!("    /// {markers}"));
        assert!(
            formatted.contains(&format!("/// {}", markers.trim_end())),
            "{formatted}"
        );
    }
}

/// A line of many quote or list markers is matched against javac's line
/// patterns once for each marker.
#[test]
fn many_markers_on_a_line() {
    for marker in [">", "> ", "- ", "1. "] {
        let formatted = format_in_time(&format!("    /// {} text", marker.repeat(100_000)));
        assert!(formatted.contains("text"), "{marker}");
    }
}

/// A Markdown paragraph on one long line is filled one word at a time.
#[test]
fn long_markdown_line() {
    let formatted = format_in_time(&format!("    /// {}", "word ".repeat(400_000)));
    assert!(formatted.lines().count() > 10_000);
}

/// Each nested HTML list indents its items further; the indentation stops
/// growing at a bound, so the output stays linear in the input and the
/// indentation never overflows.
#[test]
fn deeply_nested_html_lists() {
    let depth = 11_000;
    let comment = format!(
        "    /** {}item{} */",
        "<ul><li>".repeat(depth),
        "</li></ul>".repeat(depth)
    );
    let formatted = format_in_time(&comment);
    assert!(formatted.len() < comment.len() * 30, "{}", formatted.len());
}

/// A line with a long indentation and many text nodes between inline tags:
/// each text node asks whether it starts its line, which must not rescan the
/// indentation.
#[test]
fn many_text_nodes_after_a_long_indentation() {
    let n = 80_000;
    let comment = format!(
        "    /**\n     * heading\n     *{}{}\n     */",
        " ".repeat(n),
        "a {@code x} ".repeat(n)
    );
    let formatted = format_in_time(&comment);
    assert!(formatted.contains("{@code x}"));
}
