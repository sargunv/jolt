//! Per-token answers to the lookahead questions that would otherwise rescan
//! a bracketed group, an annotation or modifier run, or the rest of a body
//! for every member, statement, or parameter. The table is built once, in
//! linear time, when the parser is created, so every query is `O(1)`.
//!
//! Splitting a `>`-family token changes no token an entry depends on and no
//! token index, so the table stays valid for the whole parse.

use jolt_syntax::Parser;

use super::{JavaSyntaxKind, TypeModifier, starts_annotation};
use crate::JavaLanguage;

pub(in crate::parser) struct LookaheadTable {
    ends: Vec<Ends>,
}

/// Where the scans that start at one token end, as token indices.
#[derive(Clone, Copy, Default)]
struct Ends {
    /// For a `(` or `[`: the index after its matching `)` or `]`, or EOF.
    bracket: u32,
    /// The first `{`, `;`, or EOF at bracket depth zero.
    header: u32,
    /// The first `;`, `}`, or EOF at bracket depth zero, with
    /// `SAW_INVOCATION` set when a `this(` or `super(` precedes it at
    /// bracket depth zero.
    statement: u32,
    /// The end of the run of annotations starting here, or this token.
    annotations: u32,
    /// The end of the run of annotations and type modifiers starting here,
    /// or this token.
    modifiers: u32,
}

const SAW_INVOCATION: u32 = 1 << 31;

impl LookaheadTable {
    /// # Panics
    ///
    /// Panics when the token count does not fit in 31 bits.
    pub(in crate::parser) fn new(parser: &Parser<'_, JavaLanguage>) -> Self {
        let kind = |index: usize| parser.kind_at(index);
        let mut eof = 0;
        while kind(eof) != JavaSyntaxKind::Eof {
            eof += 1;
        }
        assert!(
            u32::try_from(eof).is_ok_and(|eof| eof < SAW_INVOCATION),
            "token count fits in 31 bits"
        );
        #[expect(clippy::cast_possible_truncation, reason = "checked above")]
        let at = |index: usize| index as u32;
        let mut ends = vec![
            Ends {
                bracket: at(eof),
                ..Ends::default()
            };
            eof + 1
        ];

        let (mut parens, mut brackets) = (Vec::new(), Vec::new());
        for index in 0..eof {
            let (opens, closes) = match kind(index) {
                JavaSyntaxKind::LParen => (&mut parens, false),
                JavaSyntaxKind::LBracket => (&mut brackets, false),
                JavaSyntaxKind::RParen => (&mut parens, true),
                JavaSyntaxKind::RBracket => (&mut brackets, true),
                _ => continue,
            };
            if !closes {
                opens.push(index);
            } else if let Some(open) = opens.pop() {
                ends[open].bracket = at(index + 1);
            }
        }

        // Right to left: every entry depends only on later ones.
        ends[eof] = Ends {
            bracket: at(eof),
            header: at(eof),
            statement: at(eof),
            annotations: at(eof),
            modifiers: at(eof),
        };
        for index in (0..eof).rev() {
            let current = kind(index);
            let next = match current {
                JavaSyntaxKind::LParen | JavaSyntaxKind::LBracket => ends[index].bracket as usize,
                _ => index + 1,
            };
            let header = match current {
                JavaSyntaxKind::LBrace | JavaSyntaxKind::Semicolon => at(index),
                _ => ends[next].header,
            };
            let statement = match current {
                JavaSyntaxKind::Semicolon | JavaSyntaxKind::RBrace => at(index),
                JavaSyntaxKind::ThisKw | JavaSyntaxKind::SuperKw
                    if kind(index + 1) == JavaSyntaxKind::LParen =>
                {
                    ends[next].statement | SAW_INVOCATION
                }
                _ => ends[next].statement,
            };
            let annotations = if starts_annotation(current, kind(index + 1)) {
                ends[annotation_end(parser, &ends, index)].annotations
            } else {
                at(index)
            };
            let modifiers = if annotations != at(index) {
                ends[annotations as usize].modifiers
            } else if let Some(modifier) = TypeModifier::from_tokens(
                current,
                parser.text_at(index),
                kind(index + 1),
                parser.text_at(index + 2),
            ) {
                ends[index + modifier.len()].modifiers
            } else {
                at(index)
            };
            ends[index] = Ends {
                bracket: ends[index].bracket,
                header,
                statement,
                annotations,
                modifiers,
            };
        }
        Self { ends }
    }

    /// The index after the `)` or `]` that matches the `(` or `[` at
    /// `index`, or EOF when it is unmatched.
    pub(in crate::parser) fn bracket_end(&self, index: usize) -> usize {
        self.ends[index].bracket as usize
    }

    /// The first `{`, `;`, or EOF from `index` on, skipping bracketed groups.
    pub(in crate::parser) fn header_end(&self, index: usize) -> usize {
        self.ends[index].header as usize
    }

    /// The first `;`, `}`, or EOF from `index` on, skipping bracketed groups,
    /// and whether a `this(` or `super(` precedes it outside them.
    pub(in crate::parser) fn statement_end(&self, index: usize) -> (usize, bool) {
        let entry = self.ends[index].statement;
        (
            (entry & !SAW_INVOCATION) as usize,
            entry & SAW_INVOCATION != 0,
        )
    }

    /// The end of the run of annotations that starts at `index`.
    pub(in crate::parser) fn annotations_end(&self, index: usize) -> usize {
        self.ends[index].annotations as usize
    }

    /// The end of the run of annotations and type modifiers that starts at
    /// `index`.
    pub(in crate::parser) fn modifiers_end(&self, index: usize) -> usize {
        self.ends[index].modifiers as usize
    }
}

/// The index after the annotation at `index`: `@`, a dotted name, and a
/// parenthesized argument list.
fn annotation_end(parser: &Parser<'_, JavaLanguage>, ends: &[Ends], index: usize) -> usize {
    let kind = |index: usize| parser.kind_at(index);
    let mut end = index + 1;
    if kind(end) == JavaSyntaxKind::Identifier {
        end += 1;
        while kind(end) == JavaSyntaxKind::Dot && kind(end + 1) == JavaSyntaxKind::Identifier {
            end += 2;
        }
        if kind(end) == JavaSyntaxKind::LParen {
            end = ends[end].bracket as usize;
        }
    }
    end
}

#[cfg(test)]
mod tests {
    use crate::JavaSyntaxKind;
    use crate::parser::source::Parser;

    // Fixtures prove parse results, but not that a scan the parser repeats
    // at every token is answered from the table in constant time, nor the
    // table's endpoints, which surrounding recovery can mask.

    #[test]
    fn brackets_end_after_their_match_or_at_eof() {
        // (  [  a  ]  (  b  )  )  (  c  EOF
        // 0  1  2  3  4  5  6  7  8  9  10
        let parser = Parser::new("([a](b))(c");
        let ends: Vec<_> = [0, 1, 4, 8]
            .map(|index| parser.table.bracket_end(index))
            .into();
        assert_eq!(ends, [8, 4, 7, 10]);
    }

    #[test]
    fn every_annotation_of_a_run_ends_where_the_run_does() {
        let depth = 64;
        let source = format!("{}value", "@a.b.c(x = (y)) ".repeat(depth));
        let parser = Parser::new(&source);
        let tokens_per_annotation = 13;
        let end = tokens_per_annotation * depth;
        assert_eq!(parser.kind_at(end), JavaSyntaxKind::Identifier);
        for start in (0..end).step_by(tokens_per_annotation) {
            assert_eq!(parser.table.annotations_end(start), end);
        }
        assert_eq!(parser.table.annotations_end(end), end, "no run starts here");
    }

    #[test]
    fn modifier_runs_end_at_the_first_other_token() {
        // public  @  A  non  -  sealed  static  class  @  interface
        // 0       1  2  3    4  5       6       7      8  9
        let parser = Parser::new("public @A non-sealed static class @interface");
        let ends: Vec<_> = [0, 1, 3, 6, 7, 8]
            .map(|index| parser.table.modifiers_end(index))
            .into();
        assert_eq!(ends, [7, 7, 7, 7, 7, 8]);
    }

    #[test]
    fn header_and_statement_ends_skip_bracketed_groups() {
        // this  (  ;  {  )  [  }  ]  ;  {
        // 0     1  2  3  4  5  6  7  8  9
        let parser = Parser::new("this(;{)[}];{");
        assert_eq!(parser.table.header_end(0), 8);
        assert_eq!(parser.table.statement_end(0), (8, true));
        assert_eq!(parser.table.statement_end(1), (8, false));
        assert_eq!(parser.table.header_end(9), 9);
    }

    #[test]
    fn nested_parentheses_are_never_lambda_parameters() {
        let depth = 128;
        let source = format!("{}value{}", "(".repeat(depth), ")".repeat(depth));
        let mut parser = Parser::new(&source);
        for _ in 0..depth {
            assert!(!parser.starts_parenthesized_lambda_expression());
            parser.bump();
        }
    }

    #[test]
    fn annotations_after_a_rewound_type_scan_are_skipped() {
        let parser = Parser::new("T @A value");
        let mut lookahead = parser.lookahead();
        assert!(lookahead.skip_type());
        assert!(lookahead.at(JavaSyntaxKind::At));
        assert!(lookahead.skip_annotations());
        assert_eq!(lookahead.text(), "value");
    }
}
