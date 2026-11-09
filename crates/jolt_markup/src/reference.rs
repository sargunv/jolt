//! javac's `ReferenceParser` (JDK 25): whether the signature of a reference
//! in a doc comment (`{@link Map#put(Object, Object)}`, `@throws
//! IOException`) is one javac accepts. javac reads an invalid one as an
//! error that takes the rest of the tag with it, so the reading of what
//! follows depends on it.
//!
//! `ReferenceParser` splits the signature at its first `/` (the module),
//! its first `#` after that (the member) and its first `(` after both (the
//! parameters), and reads each part with javac's own parser: `qualident`
//! for the module, `parseType` for the qualifier and each parameter, and
//! `ident` for the member. This module follows the split exactly and reads
//! the parts with the subset of javac's type grammar that references use:
//! names, primitive types, type arguments with wildcards, and arrays. An
//! annotation outside the parameters (where javac rejects it) or a comment
//! inside a signature, which references never hold in practice, is
//! [`Validity::Unknown`].

use crate::java;

/// Whether a tag's reference may name a member (`ReferenceParser.Mode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Member {
    /// A type or a member: `{@link}`, `{@linkplain}`, `@see`.
    Optional,
    /// Only a type: `@throws`, `@exception`, `@provides`, `@uses`,
    /// `{@inheritDoc}`, the type of `@serialField`.
    Disallowed,
    /// Only a member: `{@value}`.
    Required,
}

/// Whether javac accepts a signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Validity {
    Valid,
    Invalid,
    /// Holds what this port doesn't read (an annotation outside the
    /// parameters, a comment).
    Unknown,
}

/// javac's `ReferenceParser.parse`.
pub(crate) fn check(sig: &str, member: Member) -> Validity {
    match parse(sig, member) {
        Ok(()) => Validity::Valid,
        Err(validity) => validity,
    }
}

type Read = Result<(), Validity>;

fn parse(sig: &str, member: Member) -> Read {
    let slash = sig.find('/');
    let after_slash = slash.map_or(0, |slash| slash + 1);
    let hash = find_from(sig, '#', after_slash);
    let after_hash = hash.map(|hash| hash + 1);
    let lparen = find_from(sig, '(', slash.max(hash).map_or(0, |found| found + 1));
    match slash {
        Some(0) => return Err(Validity::Invalid),
        Some(slash) => Part::new(&sig[..slash], false).module()?,
        None => {}
    }
    if slash.is_some() && sig.len() == after_slash {
        // A module alone.
    } else if let (Some(hash), Some(after_hash)) = (hash, after_hash) {
        if member == Member::Disallowed {
            return Err(Validity::Invalid);
        }
        if hash != after_slash {
            Part::new(&sig[after_slash..hash], false).whole_type()?;
        }
        if find_from(sig, '#', after_hash) == Some(after_hash) {
            // `##`: a literal URL fragment.
            if member != Member::Optional {
                return Err(Validity::Invalid);
            }
        } else {
            Part::new(&sig[after_hash..lparen.unwrap_or(sig.len())], false).member()?;
        }
    } else if lparen.is_none() && member != Member::Required {
        Part::new(&sig[after_slash..], false).whole_type()?;
    } else {
        if member == Member::Disallowed {
            return Err(Validity::Invalid);
        }
        Part::new(&sig[after_slash..lparen.unwrap_or(sig.len())], false).member()?;
    }
    if let Some(lparen) = lparen {
        let rparen = find_from(sig, ')', lparen);
        if rparen != Some(sig.len() - 1) {
            return Err(Validity::Invalid);
        }
        let params = &sig[lparen + 1..sig.len() - 1];
        if !params.chars().all(java::is_whitespace) {
            Part::new(&params.replace("...", "[]"), true).params()?;
        }
    }
    Ok(())
}

/// `str::find` from byte `from` on.
fn find_from(text: &str, ch: char, from: usize) -> Option<usize> {
    text.get(from..)?.find(ch).map(|found| from + found)
}

/// A token of javac's scanner, as far as a type tells them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token<'s> {
    Ident(&'s str),
    /// A reserved word, `true`, `false`, `null` or `_`.
    Keyword(&'s str),
    Dot,
    Lt,
    Gt,
    Comma,
    Question,
    LBracket,
    RBracket,
    At,
    /// A comment, which this port doesn't read.
    Comment,
    /// Any other token, or a character javac's scanner rejects.
    Other,
    End,
}

const KEYWORDS: &[&str] = &[
    "_",
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "false",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "true",
    "try",
    "void",
    "volatile",
    "while",
];

/// The primitive types and `void`, which `parseType` accepts.
const PRIMITIVES: &[&str] = &[
    "boolean", "byte", "char", "double", "float", "int", "long", "short", "void",
];

/// The names `parseType` rejects as a type of one name
/// (`restrictedTypeName`).
const RESTRICTED: &[&str] = &["var", "yield", "record", "sealed", "permits"];

/// One part of a signature, read as javac's parser reads it.
struct Part<'s> {
    tokens: Vec<Token<'s>>,
    at: usize,
    /// The part is the parameters, where javac rejects an annotation.
    params: bool,
}

impl<'s> Part<'s> {
    fn new(text: &'s str, params: bool) -> Self {
        Self {
            tokens: tokens(text),
            at: 0,
            params,
        }
    }

    fn peek(&self) -> Token<'s> {
        self.tokens.get(self.at).copied().unwrap_or(Token::End)
    }

    fn bump(&mut self) -> Token<'s> {
        let token = self.peek();
        self.at += 1;
        token
    }

    fn expect(&mut self, token: Token<'_>) -> Read {
        if self.bump() == token {
            Ok(())
        } else {
            Err(self.failure())
        }
    }

    /// What a token the grammar doesn't take means: an error, unless the
    /// part holds a comment, or an annotation outside the parameters.
    fn failure(&self) -> Validity {
        let unread = |token: &Token<'_>| match token {
            Token::Comment => true,
            Token::At => !self.params,
            _ => false,
        };
        if self.tokens.iter().any(unread) {
            Validity::Unknown
        } else {
            Validity::Invalid
        }
    }

    fn end(&mut self) -> Read {
        self.expect(Token::End)
    }

    /// `qualident(false)`: names separated by dots.
    fn module(&mut self) -> Read {
        self.name()?;
        while self.peek() == Token::Dot {
            self.bump();
            self.name()?;
        }
        self.end()
    }

    /// `ident()`, then the part's end.
    fn member(&mut self) -> Read {
        self.name()?;
        self.end()
    }

    /// `parseType()`, then the part's end.
    fn whole_type(&mut self) -> Read {
        self.ty()?;
        self.end()
    }

    /// `parseParams`: types separated by commas, each with an optional
    /// name after it.
    fn params(&mut self) -> Read {
        loop {
            self.ty()?;
            if matches!(self.peek(), Token::Ident(_)) {
                self.bump();
            }
            if self.peek() != Token::Comma {
                break;
            }
            self.bump();
        }
        self.end()
    }

    /// `ident()`: a name that isn't a keyword.
    fn name(&mut self) -> Read {
        match self.bump() {
            Token::Ident(_) => Ok(()),
            _ => Err(self.failure()),
        }
    }

    /// `parseType()`: a primitive type or a class type, then dimensions.
    /// One name alone, or with dimensions, mustn't be a restricted name.
    fn ty(&mut self) -> Read {
        match self.bump() {
            Token::Keyword(keyword) if PRIMITIVES.contains(&keyword) => self.dims(),
            Token::Ident(name) => {
                let mut simple = !self.type_args()?;
                while self.peek() == Token::Dot {
                    self.bump();
                    self.name()?;
                    self.type_args()?;
                    simple = false;
                }
                self.dims()?;
                if simple && RESTRICTED.contains(&name) {
                    return Err(Validity::Invalid);
                }
                Ok(())
            }
            _ => Err(self.failure()),
        }
    }

    /// `typeArgumentsOpt` in a type, where `<>` is an error; whether there
    /// were any.
    fn type_args(&mut self) -> Result<bool, Validity> {
        if self.peek() != Token::Lt {
            return Ok(false);
        }
        self.bump();
        loop {
            self.type_arg()?;
            if self.peek() != Token::Comma {
                break;
            }
            self.bump();
        }
        self.expect(Token::Gt)?;
        Ok(true)
    }

    /// `typeArgument`: a type, or `?` with an optional bound.
    fn type_arg(&mut self) -> Read {
        if self.peek() != Token::Question {
            return self.ty();
        }
        self.bump();
        match self.peek() {
            Token::Keyword("extends" | "super") => {
                self.bump();
                self.ty()
            }
            Token::Ident(_) | Token::Keyword("_" | "assert" | "enum") => Err(self.failure()),
            _ => Ok(()),
        }
    }

    /// `bracketsOpt`: pairs of `[]`.
    fn dims(&mut self) -> Read {
        while self.peek() == Token::LBracket {
            self.bump();
            self.expect(Token::RBracket)?;
        }
        Ok(())
    }
}

/// javac's scanner over `text`, as far as a type tells tokens apart.
fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        let next = chars.peek().map(|&(_, next)| next);
        let token = match ch {
            ' ' | '\t' | '\u{c}' | '\n' | '\r' => continue,
            '/' if matches!(next, Some('/' | '*')) => Token::Comment,
            '.' if next == Some('.') || next.is_some_and(|next| next.is_ascii_digit()) => {
                Token::Other
            }
            '.' => Token::Dot,
            '<' => Token::Lt,
            '>' => Token::Gt,
            ',' => Token::Comma,
            '?' => Token::Question,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '@' => Token::At,
            _ if java::is_java_identifier_start(ch) => {
                let mut end = start + ch.len_utf8();
                while let Some(&(at, part)) = chars.peek() {
                    if !java::is_java_identifier_part(part) {
                        break;
                    }
                    end = at + part.len_utf8();
                    chars.next();
                }
                let word = &text[start..end];
                if KEYWORDS.contains(&word) {
                    Token::Keyword(word)
                } else {
                    Token::Ident(word)
                }
            }
            _ => Token::Other,
        };
        tokens.push(token);
    }
    tokens
}
