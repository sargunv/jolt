//! Renders the Java schema to `src/generated.rs`. Fails when the checked-in
//! file is stale; rewrites it when `INSTA_UPDATE=always`.

mod model;
mod schema;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use model::{Field, Matcher, Schema, Shape, Trailing};

#[test]
fn generated_code_is_current() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/generated.rs");
    let generated = rustfmt(&render(&schema::java()));
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    if current == generated {
        return;
    }
    if std::env::var("INSTA_UPDATE").is_ok_and(|value| value == "always") {
        std::fs::write(&path, generated).expect("write generated code");
        return;
    }
    let line = current
        .lines()
        .zip(generated.lines())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| current.lines().count().min(generated.lines().count()));
    panic!(
        "{} is stale from line {}; rerun with INSTA_UPDATE=always (mise run test --update)",
        path.display(),
        line + 1
    );
}

fn rustfmt(source: &str) -> String {
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("rustfmt must be installed");
    std::io::Write::write_all(child.stdin.as_mut().expect("stdin"), source.as_bytes())
        .expect("write to rustfmt");
    let output = child.wait_with_output().expect("run rustfmt");
    assert!(
        output.status.success(),
        "rustfmt rejected the generated code"
    );
    String::from_utf8(output.stdout).expect("rustfmt emits UTF-8")
}

/// The Rust type of a field or list item.
#[derive(Clone, PartialEq)]
enum Ty {
    Token,
    Node(&'static str),
    Category(&'static str),
    Role(&'static str),
}

#[derive(Default)]
struct Role {
    nodes: Vec<&'static str>,
    categories: Vec<&'static str>,
    token: bool,
}

struct Ctx<'s> {
    schema: &'s Schema,
    roles: BTreeMap<&'static str, Role>,
}

fn flatten(matcher: &Matcher, role: &mut Role) {
    let add = |name: &'static str, role: &mut Role| {
        if !role.nodes.contains(&name) {
            role.nodes.push(name);
        }
    };
    match matcher {
        Matcher::Token(_) | Matcher::Tokens(_) | Matcher::Contextual(_) => role.token = true,
        Matcher::Node(name) => add(name, role),
        // Token names are partitioned out by `resolve`.
        Matcher::Nodes(names) | Matcher::Elements(names) => {
            for name in names {
                add(name, role);
            }
        }
        Matcher::Category(name) => role.categories.push(name),
        Matcher::Choice(alternatives) => alternatives.iter().for_each(|m| flatten(m, role)),
    }
}

impl Ctx<'_> {
    fn is_token(&self, name: &str) -> bool {
        self.schema.tokens.contains(&name)
    }

    fn resolve(&mut self, matcher: &Matcher, role: Option<&'static str>, list_item: bool) -> Ty {
        let mut parts = Role::default();
        flatten(matcher, &mut parts);
        let (tokens, nodes): (Vec<_>, Vec<_>) =
            parts.nodes.iter().partition(|name| self.is_token(name));
        parts.token |= !tokens.is_empty();
        parts.nodes = nodes;
        match (
            parts.nodes.as_slice(),
            parts.categories.as_slice(),
            parts.token,
        ) {
            ([], [], _) => Ty::Token,
            ([node], [], false) => Ty::Node(node),
            ([], [category], false) => Ty::Category(category),
            _ => {
                let name = role.expect("multi-kind fields need a role name");
                if list_item && parts.categories.is_empty() && !parts.nodes.contains(&"Bogus") {
                    parts.nodes.push("Bogus");
                }
                self.roles.insert(name, parts);
                Ty::Role(name)
            }
        }
    }
}

fn rust_type(ty: &Ty) -> String {
    match ty {
        Ty::Token => "SyntaxToken<'a>".to_owned(),
        Ty::Node(name) | Ty::Category(name) | Ty::Role(name) => format!("{name}<'a>"),
    }
}

fn cast_element(ty: &Ty) -> String {
    match ty {
        Ty::Token => "SyntaxElement::as_token".to_owned(),
        Ty::Role(name) => format!("{name}::cast_element"),
        Ty::Node(name) | Ty::Category(name) => format!("cast::<{name}<'a>>"),
    }
}

fn condition(matcher: &Matcher) -> String {
    let set = |names: &[&str]| {
        names
            .iter()
            .map(|name| format!("K::{name}"))
            .collect::<Vec<_>>()
            .join(" | ")
    };
    match matcher {
        Matcher::Token(name) => format!("token(i, c) == Some(K::{name})"),
        Matcher::Tokens(names) => format!("matches!(token(i, c), Some({}))", set(names)),
        Matcher::Contextual(text) => format!("contextual(i, c, {text:?})"),
        Matcher::Node(name) => format!("node(i, c) == Some(K::{name})"),
        Matcher::Nodes(names) => format!("matches!(node(i, c), Some({}))", set(names)),
        Matcher::Elements(names) => format!("matches!(kind(i, c), Some({}))", set(names)),
        Matcher::Category(name) => format!("node(i, c).is_some_and(is_{})", snake(name)),
        Matcher::Choice(alternatives) => {
            let parts: Vec<_> = alternatives.iter().map(condition).collect();
            parts.join(" || ")
        }
    }
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for (index, ch) in name.char_indices() {
        if ch.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "crate",
        "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl",
        "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub",
        "ref", "return", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
        "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
    ];
    if KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_owned()
    }
}

#[allow(clippy::too_many_lines)]
fn render(schema: &Schema) -> String {
    let mut ctx = Ctx {
        schema,
        roles: BTreeMap::new(),
    };
    assert!(
        schema.tokens.len() <= 128,
        "token kinds must fit a TokenSet"
    );
    let mut nodes: Vec<&str> = schema.nodes.iter().map(|node| node.name).collect();
    nodes.push("Bogus");
    let kinds: Vec<&str> = schema
        .tokens
        .iter()
        .copied()
        .chain(nodes.iter().copied())
        .collect();
    let lists: Vec<&str> = schema
        .nodes
        .iter()
        .filter(|node| matches!(node.shape, Shape::List(_)))
        .map(|node| node.name)
        .collect();

    let mut out = String::new();
    let w = &mut out;
    w.push_str(
        "// @generated by `crates/jolt_java_syntax/tests/codegen` from its schema. Do not edit.\n\n",
    );
    w.push_str("#![allow(clippy::all, clippy::pedantic)]\n\n");
    w.push_str("use jolt_syntax::{AstNode, FactorySlot, NodeId, NodeSink, ParsedChildren, RawSyntaxKind, SyntaxElement, SyntaxFactory, SyntaxNode, SyntaxToken, ListElement, Trailing};\n\n");
    w.push_str("use crate::JavaLanguage;\n\n");

    // Kinds.
    w.push_str("/// Every Java token and node kind. Tokens come first.\n#[repr(u16)]\n#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]\npub enum JavaSyntaxKind {\n");
    for kind in &kinds {
        let _ = writeln!(w, "{kind},");
    }
    w.push_str("}\n\n");
    let _ = writeln!(w, "const TOKEN_COUNT: u16 = {};\n", schema.tokens.len());
    let _ = writeln!(w, "const KINDS: [JavaSyntaxKind; {}] = [", kinds.len());
    for kind in &kinds {
        let _ = writeln!(w, "JavaSyntaxKind::{kind},");
    }
    w.push_str("];\n\n");
    let _ = writeln!(w, "const NAMES: [&str; {}] = [", kinds.len());
    for kind in &kinds {
        let _ = writeln!(w, "{kind:?},");
    }
    w.push_str("];\n\n");
    w.push_str("impl JavaSyntaxKind {\n");
    w.push_str("pub const fn is_token(self) -> bool { (self as u16) < TOKEN_COUNT }\n");
    let _ = writeln!(
        w,
        "pub const fn is_list(self) -> bool {{ matches!(self, {}) }}",
        lists
            .iter()
            .map(|l| format!("Self::{l}"))
            .collect::<Vec<_>>()
            .join(" | ")
    );
    w.push_str("pub const fn name(self) -> &'static str { NAMES[self as usize] }\n");
    w.push_str("pub const fn to_raw(self) -> RawSyntaxKind { RawSyntaxKind(self as u16) }\n");
    w.push_str("/// # Panics\n///\n/// Panics when `raw` is not a Java kind.\npub const fn from_raw(raw: RawSyntaxKind) -> Self { KINDS[raw.0 as usize] }\n");
    w.push_str("}\n\n");
    w.push_str("use JavaSyntaxKind as K;\n\n");
    w.push_str("fn cast<'a, N: AstNode<'a>>(element: SyntaxElement<'a>) -> Option<N> { element.as_node().and_then(N::cast) }\n\n");

    // Node newtypes and accessors.
    let mut factory = String::new();
    for name in &nodes {
        let _ = writeln!(
            w,
            "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub struct {name}<'a>(SyntaxNode<'a>);\n"
        );
        let _ = writeln!(
            w,
            "impl<'a> AstNode<'a> for {name}<'a> {{\nfn can_cast(kind: RawSyntaxKind) -> bool {{ kind == K::{name}.to_raw() }}\nfn cast(node: SyntaxNode<'a>) -> Option<Self> {{ Self::can_cast(node.kind()).then_some(Self(node)) }}\nfn syntax(self) -> SyntaxNode<'a> {{ self.0 }}\n}}\n"
        );
    }
    for node in &schema.nodes {
        let name = node.name;
        match &node.shape {
            Shape::Fixed(fields) => {
                let _ = writeln!(w, "impl<'a> {name}<'a> {{");
                for (slot, field) in fields.iter().enumerate() {
                    let ty = ctx.resolve(&field.matcher, field.role, false);
                    let _ = writeln!(
                        w,
                        "pub fn {}(self) -> Option<{}> {{ self.0.slot({slot}).and_then({}) }}",
                        ident(field.name),
                        rust_type(&ty),
                        cast_element(&ty)
                    );
                }
                w.push_str("}\n\n");
                render_fixed_arm(&mut factory, name, fields);
            }
            Shape::List(list) => {
                let ty = ctx.resolve(&list.item, list.role, true);
                let item = rust_type(&ty);
                let cast = cast_element(&ty);
                let _ = writeln!(w, "impl<'a> {name}<'a> {{");
                if list.separator.is_some() {
                    let _ = writeln!(
                        w,
                        "/// Present items; `Bogus` items are skipped unless the item type holds them.\npub fn items(self) -> impl Iterator<Item = {item}> + 'a {{ self.0.slots().step_by(2).flatten().filter_map({cast}) }}"
                    );
                    let _ = writeln!(
                        w,
                        "/// Items, separators, and missing elements in order.\npub fn elements(self) -> impl Iterator<Item = ListElement<'a, {item}>> + 'a {{ self.0.slots().enumerate().map(|(index, element)| ListElement::new(index, element, {cast})) }}"
                    );
                } else {
                    let _ = writeln!(
                        w,
                        "/// Present items; `Bogus` items are skipped unless the item type holds them.\npub fn items(self) -> impl Iterator<Item = {item}> + 'a {{ self.0.slots().flatten().filter_map({cast}) }}"
                    );
                }
                w.push_str("}\n\n");
                let mut item = condition(&list.item);
                item = format!("{item} || node(i, c) == Some(K::Bogus)");
                let _ = match list.separator {
                    None => writeln!(
                        factory,
                        "K::{name} => sink.list(raw, i.len(), {}, |c| {item}),",
                        list.minimum
                    ),
                    Some((separator, trailing)) => {
                        let trailing = match trailing {
                            Trailing::Forbidden => "Forbidden",
                            Trailing::Optional => "Optional",
                            Trailing::Required => "Required",
                        };
                        writeln!(
                            factory,
                            "K::{name} => sink.separated_list(raw, i.len(), {}, Trailing::{trailing}, |c| {item}, |c| token(i, c) == Some(K::{separator})),",
                            list.minimum
                        )
                    }
                };
            }
        }
    }

    // Categories.
    for category in &schema.categories {
        let mut members = category.members.clone();
        members.push("Bogus");
        render_enum(w, category.name, &members, &[], false, false);
        let _ = writeln!(
            w,
            "fn is_{}(kind: JavaSyntaxKind) -> bool {{ matches!(kind, {}) }}\n",
            snake(category.name),
            members
                .iter()
                .map(|m| format!("K::{m}"))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    // Role enums.
    for (name, role) in &ctx.roles {
        render_enum(
            w,
            name,
            &role.nodes,
            &role.categories,
            role.token,
            role.token,
        );
    }

    // Factory.
    w.push_str("fn kind(i: ParsedChildren<'_>, c: usize) -> Option<JavaSyntaxKind> { i.kind(c).map(JavaSyntaxKind::from_raw) }\n");
    w.push_str("fn token(i: ParsedChildren<'_>, c: usize) -> Option<JavaSyntaxKind> { if i.is_token(c) { kind(i, c) } else { None } }\n");
    w.push_str("fn node(i: ParsedChildren<'_>, c: usize) -> Option<JavaSyntaxKind> { if i.is_node(c) { kind(i, c) } else { None } }\n");
    w.push_str("fn contextual(i: ParsedChildren<'_>, c: usize, text: &str) -> bool { token(i, c) == Some(K::Identifier) && i.token_text(c).is_some_and(|actual| crate::lexer::lexical_text_is(actual, text)) }\n\n");
    w.push_str("pub(crate) struct JavaSyntaxFactory;\n\nimpl SyntaxFactory for JavaSyntaxFactory {\ntype Language = JavaLanguage;\n\nfn make(&self, raw: RawSyntaxKind, i: ParsedChildren<'_>, sink: &mut NodeSink<'_>) -> NodeId {\nmatch JavaSyntaxKind::from_raw(raw) {\n");
    w.push_str(&factory);
    w.push_str("_ => sink.raw(raw, true),\n}\n}\n}\n");
    out
}

fn render_fixed_arm(out: &mut String, name: &str, fields: &[Field]) {
    let _ = writeln!(out, "K::{name} => {{\nlet mut c = 0;");
    for (slot, field) in fields.iter().enumerate() {
        let missing = if field.required { "Missing" } else { "Absent" };
        let _ = writeln!(
            out,
            "let s{slot} = if {} {{ c += 1; FactorySlot::Child(c - 1) }} else {{ FactorySlot::{missing} }};",
            condition(&field.matcher)
        );
    }
    let slots: Vec<_> = (0..fields.len()).map(|slot| format!("s{slot}")).collect();
    let _ = writeln!(
        out,
        "if c == i.len() {{ sink.fixed(raw, [{}]) }} else {{ sink.raw(raw, true) }}\n}}",
        slots.join(", ")
    );
}

/// An enum over node kinds, categories, and (optionally) tokens.
fn render_enum(
    w: &mut String,
    name: &str,
    nodes: &[&str],
    categories: &[&str],
    token: bool,
    element_only: bool,
) {
    let _ = writeln!(
        w,
        "#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\npub enum {name}<'a> {{"
    );
    for variant in nodes.iter().chain(categories) {
        let _ = writeln!(w, "{variant}({variant}<'a>),");
    }
    if token {
        w.push_str("Token(SyntaxToken<'a>),\n");
    }
    w.push_str("}\n\n");
    let mut node_arms = String::new();
    for variant in nodes {
        let _ = writeln!(
            node_arms,
            "K::{variant} => Self::{variant}({variant}(node)),"
        );
    }
    let mut category_arms = String::new();
    let mut category_can_cast = String::new();
    for category in categories {
        let _ = writeln!(
            category_arms,
            "_ if {category}::can_cast(node.kind()) => Self::{category}({category}::cast(node)?),"
        );
        let _ = write!(category_can_cast, " || {category}::can_cast(kind)");
    }
    let mut syntax_arms = String::new();
    for variant in nodes.iter().chain(categories) {
        let _ = writeln!(syntax_arms, "Self::{variant}(node) => node.syntax(),");
    }
    let can_cast = nodes
        .iter()
        .map(|variant| format!("K::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let can_cast = if can_cast.is_empty() {
        "false".to_owned()
    } else {
        format!("matches!(K::from_raw(kind), {can_cast})")
    };
    if element_only {
        let _ = writeln!(
            w,
            "impl<'a> {name}<'a> {{\npub fn cast_element(element: SyntaxElement<'a>) -> Option<Self> {{\nlet node = match element {{\nSyntaxElement::Token(token) => return Some(Self::Token(token)),\nSyntaxElement::Node(node) => node,\n}};\nSome(match K::from_raw(node.kind()) {{\n{node_arms}{category_arms}_ => return None,\n}})\n}}\n\npub fn element(self) -> SyntaxElement<'a> {{\nmatch self {{\n{}Self::Token(token) => SyntaxElement::Token(token),\n}}\n}}\n}}\n",
            syntax_arms.replace("=> node.syntax()", "=> SyntaxElement::Node(node.syntax())")
        );
    } else {
        let _ = writeln!(
            w,
            "impl<'a> AstNode<'a> for {name}<'a> {{\nfn can_cast(kind: RawSyntaxKind) -> bool {{ {can_cast}{category_can_cast} }}\nfn cast(node: SyntaxNode<'a>) -> Option<Self> {{\nSome(match K::from_raw(node.kind()) {{\n{node_arms}{category_arms}_ => return None,\n}})\n}}\nfn syntax(self) -> SyntaxNode<'a> {{\nmatch self {{\n{syntax_arms}}}\n}}\n}}\n"
        );
        let _ = writeln!(
            w,
            "impl<'a> {name}<'a> {{\npub fn cast_element(element: SyntaxElement<'a>) -> Option<Self> {{ element.as_node().and_then(Self::cast) }}\n}}\n"
        );
    }
}
