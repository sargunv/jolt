# Jolt Java Formatter Implementation Spec

This spec translates the Jolt Java style guide into implementation work. The
style guide defines the output policy; this document defines how to build that
policy into code without reintroducing formatter-compatibility debt.

Primary policy inputs:

- [`java-format-style.md`](java-format-style.md)
- [`java-format-style-program-comments.md`](java-format-style-program-comments.md)
- [`java-format-style-declarations.md`](java-format-style-declarations.md)
- [`java-format-style-statements.md`](java-format-style-statements.md)
- [`java-format-style-expressions.md`](java-format-style-expressions.md)

## Build Philosophy

- Build layer by layer, not by proof-grade vertical slices.
- Start by writing failing tests that pin the style guide.
- Add missing CST accessors next.
- Add required layout helpers next.
- Then write rules using those helpers.
- If a layer becomes unwieldy, rethink the layer. Do not tunnel through it with
  printer-local child scans or one-off formatting branches.
- Keep rendering bounded and linear. Do not add best-fitting, conditional-group,
  marker-column, or compatibility-only IR primitives for Java layout.

## Target Layering

```text
source text
  -> jolt_java_syntax lexer/parser
  -> lossless Java CST
  -> typed CST wrappers and accessors
  -> jolt_java_fmt rule layer
  -> jolt_java_fmt helper layer
  -> jolt_fmt_ir document
  -> jolt_fmt_ir renderer
  -> formatted source
```

Ownership:

- `jolt_java_syntax` owns parsing, CST shape, trivia, and typed accessors.
- `jolt_java_fmt` owns Java policy, rule dispatch, comment placement, import
  sorting, and CST-to-document layout.
- `jolt_fmt_ir` owns language-agnostic document construction and rendering.
- `jolt_fmt_core` owns public options, diagnostics, and engine wiring.

The Java formatter should not reach into raw syntax structure when a stable
grammar-role accessor can exist. The syntax crate should not know formatting
policy.

## Proposed Module Shape

Keep module names boring and policy-shaped:

```text
crates/jolt_java_fmt/src/
  lib.rs
  context.rs
  format.rs
  rules/
    mod.rs
    program.rs
    imports.rs
    modules.rs
    comments.rs
    names.rs
    declarations.rs
    types.rs
    statements.rs
    expressions.rs
  helpers/
    mod.rs
    blocks.rs
    lists.rs
    annotations.rs
    modifiers.rs
    comments.rs
    chains.rs
    operators.rs
    literals.rs
```

This split may change as implementation teaches us more, but dependency
direction should stay stable:

```text
rules -> helpers -> jolt_fmt_ir
rules -> jolt_java_syntax accessors
helpers must not inspect parser internals
```

## Rule Authoring Shape

Rules should read like direct translations of the style guide. They should not
assemble output strings, and they should not scan raw children unless the code
is being moved into a syntax accessor.

Representative shape:

```rust
trait FormatRule<N> {
    fn fmt(&self, node: &N, f: &mut JavaFormatter<'_>) -> Doc;
}

struct JavaFormatter<'a> {
    options: &'a FormatOptions,
    comments: &'a CommentMap,
}
```

Representative dispatch:

```rust
impl JavaFormatter<'_> {
    fn format_node(&mut self, node: AnyJavaNode) -> Doc {
        match node {
            AnyJavaNode::CompilationUnit(node) => ProgramRule.fmt(&node, self),
            AnyJavaNode::ClassDeclaration(node) => DeclarationRule.fmt(&node, self),
            AnyJavaNode::IfStatement(node) => IfRule.fmt(&node, self),
            AnyJavaNode::BinaryExpression(node) => BinaryRule.fmt(&node, self),
            _ => todo_rule(node),
        }
    }
}
```

Representative declaration rule:

```rust
impl FormatRule<ClassDeclaration> for DeclarationRule {
    fn fmt(&self, class: &ClassDeclaration, f: &mut JavaFormatter<'_>) -> Doc {
        let header = declaration_header()
            .modifiers(format_modifiers(class.modifiers(), f))
            .keyword("class")
            .name(required_token(class.name()))
            .type_parameters(
                class
                    .type_parameters()
                    .map(|p| format_type_parameters(&p, f)),
            )
            .clause(class.extends_clause().map(|c| format_extends_clause(&c, f)))
            .clause(
                class
                    .implements_clause()
                    .map(|c| format_implements_clause(&c, f)),
            )
            .clause(class.permits_clause().map(|c| format_permits_clause(&c, f)))
            .finish();

        declaration_with_body(header, class.body().map(|body| format_class_body(&body, f)))
    }
}
```

Representative helper:

```rust
fn parenthesized_list(items: Vec<Doc>) -> Doc {
    if items.is_empty() {
        return text("()");
    }

    group(concat([
        text("("),
        indent(concat([line(), join(concat([text(","), line()]), items)])),
        line(),
        text(")"),
    ]))
}
```

The exact Rust API can differ. The important property is that rules compose
named helpers instead of repeatedly encoding low-level line and indent
mechanics.

## Test Strategy

Write the failing tests first.

### Rule Fixtures

Create focused input/expected fixtures for every style-guide rule:

```text
crates/jolt_java_fmt/tests/style/
  program/
  imports/
  modules/
  comments/
  declarations/
  statements/
  expressions/
```

Each case should have:

```text
name.input.java
name.expected.java
```

Tests must fail if required fixtures are missing.

Each rule fixture should assert:

- formatting succeeds,
- formatted output equals `expected.java`,
- formatting the expected output is unchanged,
- repeated formatting is deterministic.

Snapshot tests are fine for summaries, but rule fixtures should keep expected
Java output visible in files. This makes style review concrete.

### Fixture Corpus Tests

The imported upstream fixture inputs are broad coverage. They should assert:

- parser accepts expected valid inputs,
- formatter does not crash,
- formatter output parses,
- formatter output is idempotent,
- repeated formatting is deterministic.

They must not score against google-java-format, Palantir, Prettier-Java, or
ktfmt outputs.

### Test Writing Order

1. Program/import/module/name/comment fixtures.
2. Declaration/type/parameter fixtures.
3. Statement/block/switch/try fixtures.
4. Expression/operator/call/chain/lambda fixtures.
5. Imported-corpus idempotence once enough rules exist to avoid fallback output.

Do not implement one perfect vertical fixture before writing the rest of the
rule fixtures. The first pass should expose the whole style surface.

## Accessor Discipline

Add CST accessors whenever a formatting rule needs a grammar role.

Add an accessor when code asks:

- What is this node's name, type, body, condition, or receiver?
- Is this annotation declaration-level or type-use?
- Is this import static or starred?
- What directive kind is this module directive?
- Is this statement body a block, empty semicolon, or unbraced statement?
- Is this expression the receiver of a member chain?
- Is this `_` an unnamed variable, unnamed pattern, or lambda parameter?
- Is this comment leading, trailing, or dangling for this construct?

Do not solve those questions in `jolt_java_fmt` by scanning child indexes.

Acceptable accessor:

```rust
impl MethodDeclaration {
    pub fn throws_clause(&self) -> Option<ThrowsClause> {
        child(&self.syntax)
    }
}
```

Discouraged formatter code:

```rust
let throws = method.syntax().children().find(|child| child.kind() == Throws);
```

Accessors should return typed wrappers, tokens, or semantic structs. They should
not return formatting documents.

## Comment And Trivia Model

Implement comments as formatter-owned placement over syntax-owned trivia:

```text
jolt_java_syntax:
  raw comments, token/source spans, trivia positions

jolt_java_fmt:
  CommentMap
  leading/trailing/dangling classification
  construct-specific placement
```

Do not port Prettier's global comment attachment engine. Use the same vocabulary
only where it helps.

Comment placement must be explicit for constructs that move code:

- sorted imports,
- sorted module directives,
- braced formerly-unbraced bodies,
- removed empty statements,
- normalized qualified-name dots,
- broken member chains.

Comments between sortable imports or module directives are barriers for v1.

## Helper Organization

Helpers are reusable only when they express a real policy or repeated document
shape. Do not add convenience APIs that simply rename one IR function.

Core helpers to build before rules:

- `comma_list`
- `semicolon_list`
- `parenthesized_list`
- `angle_bracket_list`
- `braced_block`
- `declaration_header`
- `member_body`
- `modifier_list`
- `annotation_group`
- `assignment_rhs`
- `binary_chain`
- `ternary_expression`
- `argument_list`
- `member_chain`
- `qualified_name`
- `line_comment`
- `block_comment`
- `star_block_comment`

Helpers should usually accept already-formatted child docs plus small policy
enums, not raw syntax nodes. Rules own syntax access; helpers own layout shape.

Example:

```rust
enum TrailingSeparator {
    Never,
    WhenBroken,
    AlwaysWhenMultiline,
}

fn comma_list(items: Vec<Doc>, trailing: TrailingSeparator) -> Doc;
```

Avoid helpers that combine unrelated concepts:

```rust
fn format_any_parenthesized_thing(node: AnyJavaNode) -> Doc;
```

Prefer focused helpers:

```rust
fn parenthesized_list(items: Vec<Doc>) -> Doc;
fn parenthesized_condition(condition: Doc) -> Doc;
fn resource_specification(resources: Vec<Doc>) -> Doc;
```

## Rule Implementation Order

The implementation order should follow dependency layers, not end-to-end
vertical slices.

### 1. Formatting Harness

- Add `jolt_java_fmt::format_source`.
- Wire Java formatting through `jolt_fmt_core::format_source`.
- Convert `FormatOptions` to explicit `RenderOptions`.
- Keep parse-error no-write behavior in the shared diagnostic policy.
- Add tests that Java formatting no longer returns the unimplemented diagnostic
  once the layout builder is enabled.

### 2. Rule Fixture Harness

- Add the input/expected fixture runner.
- Add idempotence and determinism assertions.
- Add the style-guide fixture files, even while they fail.

### 3. Accessor Pass

Add accessors required by the fixtures before writing printer workarounds:

- compilation-unit item order,
- import static/star/name/comments,
- module directive kind/name/targets/comments,
- modifier keyword and annotation roles,
- declaration headers and bodies,
- statement body kind,
- switch labels/rules/guards,
- for init/condition/update,
- try resources and optional trailing semicolon,
- expression parent roles,
- member-chain linearization,
- lambda parameter classification,
- wildcard and unnamed `_` roles.

### 4. Low-Level Helpers

Implement and test helper behavior independent of Java syntax where possible:

- broken closing delimiter on its own line,
- trailing separator policies,
- body blank-line capping,
- leading-operator chains with flat continuation alignment,
- break-all argument lists,
- Ruff-shaped member-chain heads.

### 5. Program Layer

Implement:

- final newline,
- package declarations,
- import sorting with comment barriers,
- module directive sorting/grouping,
- redundant top-level semicolon removal,
- qualified names,
- literal/token leaves.

### 6. Declaration Layer

Implement:

- modifier sorting,
- declaration/type-use annotation placement,
- class/interface/record/enum/annotation headers,
- broken-header brace placement,
- body member category padding,
- parameter and record-component lists,
- `throws`,
- enum constants and trailing comma policy,
- type-body empty statement removal.

### 7. Statement Layer

Implement:

- blocks and braced bodies,
- labels,
- if/else,
- loops and broken `for` headers,
- switch labels/rules/guards,
- return/throw/yield,
- try/catch/finally/resources,
- assert/break/continue.

### 8. Expression Layer

Implement:

- parenthesized expressions,
- binary/operator chains,
- ternaries,
- assignments,
- calls and break-all argument lists,
- member chains,
- lambdas,
- arrays/initializers,
- casts, instanceof, patterns, object creation, and type arguments.

### 9. Comments And Ignore Hardening

After the main syntax surface works:

- classify leading/trailing/dangling comments,
- place comments for constructs that move code,
- normalize star-block comments,
- preserve `@formatter:off/on` raw ranges,
- preserve text-block internals exactly,
- add imported-corpus comment/idempotence tests.

This is not permission to ignore comments earlier. It means the first formatter
can use conservative placement while policy-specific hardening lands in a
dedicated layer.

## Representative Fixtures

Import sorting:

```java
// input
import static z.Z.z;
import b.B;
import a.A;
```

```java
// expected
import a.A;
import b.B;

import static z.Z.z;
```

Broken parameters and `throws`:

```java
class A {
  Result compute(
      Request request,
      ExecutionContext context
  )
      throws IOException,
      TimeoutException
  {
    return executor.run(request, context);
  }
}
```

Leading binary operators:

```java
boolean allowed =
    user.isActive()
    && account.hasPermission("write")
    && featureFlags.enabled();
```

Member chains:

```java
ImmutableList.builder()
    .add(first)
    .add(second)
    .build();
```

## Handling Layer Friction

When a rule is hard to express, diagnose the layer:

- Missing grammar role? Add an accessor.
- Repeated line/indent shape? Add or refine a helper.
- Helper has too many flags? Split it.
- Rule requires arbitrary layout alternatives? Re-check the style guide before
  adding IR.
- Parser accepts syntax but formatter cannot format it? Add a real rule or move
  invalidation to parsing/diagnostics.

Do not add:

- fallback exits for parser-accepted syntax,
- compatibility branches for upstream formatter output,
- formatter-local parser workarounds,
- unbounded layout search,
- public options for unresolved policy debates.

## Definition Of Done

The Java layout builder is ready for broad use when:

- all style-guide rule fixtures pass,
- every style-guide rule has one or more focused tests that pin the intended
  output,
- formatting expected fixtures is idempotent,
- imported Java fixture inputs format without formatter panics,
- formatted imported fixtures parse,
- repeated formatting is deterministic,
- no parser-accepted syntax reaches an unimplemented formatter fallback,
- code review can trace every formatting choice to the style guide or this spec,
- an audit report exists that evaluates every definition-of-done item above and
  links to the relevant tests, fixture suites, or implementation evidence.
