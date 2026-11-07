//! The schema vocabulary and its builder functions.

pub struct Schema {
    /// Token kinds, in kind order. `Eof` comes first.
    pub tokens: Vec<&'static str>,
    pub categories: Vec<Category>,
    /// Node kinds, in kind order. `Bogus` is implicit and comes last.
    pub nodes: Vec<Node>,
}

/// A set of node kinds accepted at one slot. Every category also accepts
/// `Bogus`.
pub struct Category {
    pub name: &'static str,
    pub members: Vec<&'static str>,
}

pub struct Node {
    pub name: &'static str,
    pub shape: Shape,
}

pub enum Shape {
    Fixed(Vec<Field>),
    List(List),
}

pub struct Field {
    pub name: &'static str,
    pub required: bool,
    pub matcher: Matcher,
    /// The name of the enum generated for a multi-kind field.
    pub role: Option<&'static str>,
}

pub struct List {
    pub minimum: usize,
    pub item: Matcher,
    pub role: Option<&'static str>,
    pub separator: Option<(&'static str, Trailing)>,
}

#[derive(Clone, Copy)]
pub enum Trailing {
    Forbidden,
    Optional,
    Required,
}

#[derive(Clone)]
pub enum Matcher {
    Token(&'static str),
    Tokens(Vec<&'static str>),
    /// An identifier token with this text.
    Contextual(&'static str),
    Node(&'static str),
    Nodes(Vec<&'static str>),
    /// Node or token kinds.
    Elements(Vec<&'static str>),
    Category(&'static str),
    Choice(Vec<Matcher>),
}

pub fn category(name: &'static str, members: &[&'static str]) -> Category {
    Category {
        name,
        members: members.to_vec(),
    }
}

pub fn node(name: &'static str, fields: Vec<Field>) -> Node {
    Node {
        name,
        shape: Shape::Fixed(fields),
    }
}

pub fn list(name: &'static str, minimum: usize, item: Matcher) -> Node {
    Node {
        name,
        shape: Shape::List(List {
            minimum,
            item,
            role: None,
            separator: None,
        }),
    }
}

impl Node {
    pub fn role(mut self, role: &'static str) -> Self {
        let Shape::List(list) = &mut self.shape else {
            panic!("only list nodes take a role here");
        };
        list.role = Some(role);
        self
    }

    pub fn separated(mut self, separator: &'static str, trailing: Trailing) -> Self {
        let Shape::List(list) = &mut self.shape else {
            panic!("only list nodes are separated");
        };
        list.separator = Some((separator, trailing));
        self
    }
}

impl Field {
    pub fn role(mut self, role: &'static str) -> Self {
        self.role = Some(role);
        self
    }
}

pub fn req(name: &'static str, matcher: Matcher) -> Field {
    Field {
        name,
        required: true,
        matcher,
        role: None,
    }
}

pub fn opt(name: &'static str, matcher: Matcher) -> Field {
    Field {
        name,
        required: false,
        matcher,
        role: None,
    }
}

pub fn t(kind: &'static str) -> Matcher {
    Matcher::Token(kind)
}

pub fn tokens(kinds: &[&'static str]) -> Matcher {
    Matcher::Tokens(kinds.to_vec())
}

pub fn kw(text: &'static str) -> Matcher {
    Matcher::Contextual(text)
}

pub fn n(kind: &'static str) -> Matcher {
    Matcher::Node(kind)
}

pub fn nodes(kinds: &[&'static str]) -> Matcher {
    Matcher::Nodes(kinds.to_vec())
}

pub fn elements(kinds: &[&'static str]) -> Matcher {
    Matcher::Elements(kinds.to_vec())
}

pub fn cat(name: &'static str) -> Matcher {
    Matcher::Category(name)
}

pub fn choice(alternatives: Vec<Matcher>) -> Matcher {
    Matcher::Choice(alternatives)
}
