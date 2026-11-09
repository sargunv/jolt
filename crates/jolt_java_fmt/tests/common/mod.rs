// Shared by several test binaries and examples, each of which uses a part.
#![allow(dead_code)]

use jolt_fmt::FormatOptions;
use jolt_java_syntax::{JavaLanguage, JavaSyntaxKind as K};
use jolt_syntax::{Diagnostic, SyntaxNode, SyntaxToken};
use jolt_test_support::{CorpusLanguage, Depth, StructurePolicy, Suite};

/// The sweep's suites (`mise run hunt`, replayed by `jcheck`): the repository
/// fixtures, the small upstream fixture suites and the Markdown spec examples
/// (`commonmark-spec`) exhaustively, Spring and the JDK slice (`openjdk`)
/// sampled.
pub(crate) const SWEEP_SUITES: &[Suite<'_>] = &[
    Suite {
        dir: "fixtures/java",
        depth: Depth::Exhaustive,
    },
    Suite {
        dir: "tools/import/.imports/google-java-format/input",
        depth: Depth::Exhaustive,
    },
    Suite {
        dir: "tools/import/.imports/palantir-java-format/input",
        depth: Depth::Exhaustive,
    },
    Suite {
        dir: "tools/import/.imports/prettier-java/input",
        depth: Depth::Exhaustive,
    },
    Suite {
        dir: "tools/import/.imports/commonmark-spec/input",
        depth: Depth::Exhaustive,
    },
    Suite {
        dir: "tools/import/.imports/spring-framework",
        depth: Depth::Sampled,
    },
    Suite {
        dir: "tools/import/.imports/openjdk",
        depth: Depth::Sampled,
    },
];

/// The tree edits the Java formatter may make (N1–N8): import, module
/// directive, `requires` modifier and declaration modifier reordering, brace
/// promotion, removal of redundant `;` and parentheses, and separator, brace
/// and parenthesis punctuation.
pub(crate) const STRUCTURE_POLICY: StructurePolicy<K> = StructurePolicy {
    normalizable_punctuation: &[
        K::Comma,
        K::Semicolon,
        K::LBrace,
        K::RBrace,
        K::LParen,
        K::RParen,
    ],
    unordered_nodes: &[K::ModuleDirectiveList, K::RequiresModifierList],
    unordered_keywords: &[K::ModifierList],
    unordered_keywords_ordered_children: &[K::Annotation],
    reorderable_children: &[K::ImportDeclaration],
    elidable_wrappers: &[
        K::BlockStatementList,
        K::BlockStatement,
        K::ParenthesizedExpression,
    ],
    promoted_body_wrapper: Some(K::Block),
    brace_promoting_parents: &[
        K::IfStatement,
        K::WhileStatement,
        K::DoStatement,
        K::BasicForStatement,
        K::EnhancedForStatement,
    ],
    elidable_nodes: &[K::EmptyStatement, K::EmptyDeclaration],
    splittable_tokens: &[K::Gt, K::RShift, K::UnsignedRShift],
};

pub(crate) struct JavaCorpus;

impl CorpusLanguage for JavaCorpus {
    type Syntax = JavaLanguage;

    fn name(&self) -> &'static str {
        "Java"
    }

    fn extension(&self) -> &'static str {
        "java"
    }

    fn with_parse<R>(
        &self,
        source: &str,
        visit: impl FnOnce(SyntaxNode<'_>, &[Diagnostic]) -> R,
    ) -> R {
        let parse = jolt_java_syntax::parse(source);
        visit(parse.syntax(), parse.diagnostics())
    }

    fn format(&self, source: &str, options: &FormatOptions) -> String {
        format(source, *options)
    }

    fn structure_policy(&self) -> &StructurePolicy<K> {
        &STRUCTURE_POLICY
    }

    fn expects_diagnostics(&self, relative: &str) -> bool {
        let Some(name) = relative.strip_prefix("syntax/parser/") else {
            return false;
        };
        name.starts_with("diagnoses-")
            || name.starts_with("recovers-")
            || name == "disambiguates-when-in-switch-labels--invalid-guard.java"
    }

    /// JEP 467 and javadoc: a module, a package in a file without types
    /// (`package-info.java`), a type that is not local, or a member, of which
    /// `token` is the first token.
    fn starts_documented_declaration(&self, token: SyntaxToken<'_>) -> bool {
        let mut node = Some(token.parent());
        // Only the nodes that start at `token` are climbed.
        while let Some(declaration) = node.filter(|node| node.first_token() == Some(token)) {
            match K::from_raw(declaration.kind()) {
                K::PackageDeclaration => {
                    let has_types = declaration.parent().is_some_and(|items| {
                        items.children().any(|item| {
                            item.as_node().is_some_and(|item| {
                                matches!(
                                    K::from_raw(item.kind()),
                                    K::ClassDeclaration
                                        | K::InterfaceDeclaration
                                        | K::EnumDeclaration
                                        | K::RecordDeclaration
                                        | K::AnnotationInterfaceDeclaration
                                )
                            })
                        })
                    });
                    return !has_types;
                }
                K::ModuleDeclaration
                | K::ClassDeclaration
                | K::InterfaceDeclaration
                | K::EnumDeclaration
                | K::RecordDeclaration
                | K::AnnotationInterfaceDeclaration
                | K::FieldDeclaration
                | K::MethodDeclaration
                | K::ConstructorDeclaration
                | K::CompactConstructorDeclaration
                | K::EnumConstant
                | K::AnnotationElementDeclaration => {
                    let local = declaration.parent().is_some_and(|parent| {
                        K::from_raw(parent.kind()) == K::LocalClassOrInterfaceDeclaration
                    });
                    return !local;
                }
                K::LocalClassOrInterfaceDeclaration => return false,
                _ => node = declaration.parent(),
            }
        }
        false
    }
}

pub(crate) fn format(source: &str, options: FormatOptions) -> String {
    let parse = jolt_java_syntax::parse(source);
    let mut output = String::new();
    let _ = jolt_java_fmt::format(&parse, &options, &mut output);
    output
}
