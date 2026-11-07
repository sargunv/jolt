// Answers the header scans that would otherwise rescan the rest of a body for
// every member, statement, or parameter. Each table is built once, right to
// left, from the first query's position to EOF, so every query is O(1).
use super::{JavaSyntaxKind, Parser};

#[derive(Default)]
pub(in crate::parser) struct ScanTables {
    base: usize,
    /// Per token: the first `{`, `;`, or EOF at bracket depth zero.
    header_end: Vec<u32>,
    /// Per token: the first `;`, `}`, or EOF at bracket depth zero, with the
    /// high bit set when a `this(` or `super(` precedes it at depth zero.
    statement_end: Vec<u32>,
    /// Per token: whether `this` appears before the next `,`, `)`, `;`, or EOF.
    receiver: Vec<bool>,
}

const SAW_INVOCATION: u32 = 1 << 31;

/// Tokens that may appear inside type arguments.
const fn is_type_token(kind: JavaSyntaxKind) -> bool {
    matches!(
        kind,
        JavaSyntaxKind::Identifier
            | JavaSyntaxKind::Dot
            | JavaSyntaxKind::Comma
            | JavaSyntaxKind::Question
            | JavaSyntaxKind::ExtendsKw
            | JavaSyntaxKind::SuperKw
            | JavaSyntaxKind::Amp
            | JavaSyntaxKind::At
            | JavaSyntaxKind::LBracket
            | JavaSyntaxKind::RBracket
    ) || super::is_primitive_type_start(kind)
}

impl ScanTables {
    fn build(&mut self, parser: &Parser<'_>, base: usize) {
        let mut kinds = Vec::new();
        for index in base.. {
            let kind = parser.kind_at(index);
            kinds.push(kind);
            if kind == JavaSyntaxKind::Eof {
                break;
            }
        }
        let eof = kinds.len() - 1;
        // The index after the `)` or `]` matching an opener, or EOF.
        let mut after = vec![eof; kinds.len()];
        let (mut parens, mut brackets) = (Vec::new(), Vec::new());
        for (index, kind) in kinds.iter().enumerate() {
            match kind {
                JavaSyntaxKind::LParen => parens.push(index),
                JavaSyntaxKind::LBracket => brackets.push(index),
                JavaSyntaxKind::RParen => {
                    if let Some(open) = parens.pop() {
                        after[open] = index + 1;
                    }
                }
                JavaSyntaxKind::RBracket => {
                    if let Some(open) = brackets.pop() {
                        after[open] = index + 1;
                    }
                }
                _ => {}
            }
        }

        // The index after the `>` closing a type-argument `<`, for `<` runs
        // made only of type tokens (a `>>` or `>>>` closes two or three).
        let mut after_angle = vec![None; kinds.len()];
        let mut angles: Vec<usize> = Vec::new();
        for (index, &kind) in kinds.iter().enumerate() {
            let closes = match kind {
                JavaSyntaxKind::Lt => {
                    angles.push(index);
                    continue;
                }
                JavaSyntaxKind::Gt => 1,
                JavaSyntaxKind::RShift => 2,
                JavaSyntaxKind::UnsignedRShift => 3,
                kind if is_type_token(kind) => continue,
                _ => {
                    angles.clear();
                    continue;
                }
            };
            for _ in 0..closes {
                if let Some(open) = angles.pop() {
                    after_angle[open] = Some(index + 1);
                }
            }
        }

        let len = kinds.len();
        let mut header_end = vec![0u32; len];
        let mut statement_end = vec![0u32; len];
        let mut receiver = vec![false; len];
        let at = |index: usize| u32::try_from(base + index).expect("token index fits in u32");
        for index in (0..len).rev() {
            let kind = kinds[index];
            let skip = matches!(kind, JavaSyntaxKind::LParen | JavaSyntaxKind::LBracket)
                .then_some(after[index]);
            header_end[index] = match (kind, skip) {
                (JavaSyntaxKind::Eof | JavaSyntaxKind::LBrace | JavaSyntaxKind::Semicolon, _) => {
                    at(index)
                }
                (_, Some(next)) => header_end[next],
                _ => header_end[index + 1],
            };
            statement_end[index] = match (kind, skip) {
                (JavaSyntaxKind::Eof | JavaSyntaxKind::Semicolon | JavaSyntaxKind::RBrace, _) => {
                    at(index)
                }
                (_, Some(next)) => statement_end[next],
                _ => {
                    let invocation =
                        matches!(kind, JavaSyntaxKind::ThisKw | JavaSyntaxKind::SuperKw)
                            && kinds[index + 1] == JavaSyntaxKind::LParen;
                    statement_end[index + 1] | if invocation { SAW_INVOCATION } else { 0 }
                }
            };
            receiver[index] = match kind {
                JavaSyntaxKind::Eof
                | JavaSyntaxKind::Comma
                | JavaSyntaxKind::RParen
                | JavaSyntaxKind::Semicolon => false,
                JavaSyntaxKind::ThisKw => true,
                // Commas inside type arguments do not end the parameter.
                JavaSyntaxKind::Lt => {
                    after_angle[index].map_or(receiver[index + 1], |after| receiver[after])
                }
                _ => receiver[index + 1],
            };
        }
        self.base = base;
        self.header_end = header_end;
        self.statement_end = statement_end;
        self.receiver = receiver;
    }
}

impl Parser<'_> {
    /// The tables' offset for the current token, building them on first use
    /// (or again if the parser stepped back before their base).
    fn scan_offset(&mut self) -> usize {
        let position = self.position();
        if self.scan_tables.header_end.is_empty() || position < self.scan_tables.base {
            let mut tables = std::mem::take(&mut self.scan_tables);
            tables.build(self, position);
            self.scan_tables = tables;
        }
        position - self.scan_tables.base
    }

    /// Whether the member header at the current token reaches a `{` before a
    /// `;` (bracketed groups skipped).
    pub(in crate::parser::grammar) fn member_header_ends_with_block(&mut self) -> bool {
        let offset = self.scan_offset();
        let end = self.scan_tables.header_end[offset] as usize;
        self.kind_at(end) == JavaSyntaxKind::LBrace
    }

    /// Whether the statement at the current token is an explicit constructor
    /// invocation: a `this(` or `super(` before its `;`.
    pub(in crate::parser::grammar) fn starts_constructor_invocation_statement(&mut self) -> bool {
        if matches!(
            self.current(),
            JavaSyntaxKind::LBrace | JavaSyntaxKind::RParen | JavaSyntaxKind::RBracket
        ) {
            return false;
        }
        let offset = self.scan_offset();
        let entry = self.scan_tables.statement_end[offset];
        entry & SAW_INVOCATION != 0
            && self.kind_at((entry & !SAW_INVOCATION) as usize) == JavaSyntaxKind::Semicolon
    }

    /// Whether `this` appears before the end of the current parameter.
    pub(in crate::parser::grammar) fn starts_receiver_parameter(&mut self) -> bool {
        let offset = self.scan_offset();
        self.scan_tables.receiver[offset]
    }
}
