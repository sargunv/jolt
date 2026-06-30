use crate::{
    JavaFormatOptions, JavaFormatProfile, JavaFormatStatus, format_java_source,
    format_java_source_with_options,
};
use jolt_diagnostics::DiagnosticStage;
use jolt_fmt_ir::RenderOptions;

fn assert_formatted(source: &str, expected: &str) {
    assert_formatted_with_width(source, expected, 100);
}

#[cfg(test)]
fn assert_formatted_with_width(source: &str, expected: &str, line_width: u32) {
    let result = format_java_source_with_options(
        source,
        JavaFormatOptions {
            profile: JavaFormatProfile::Google,
            render: RenderOptions {
                line_width: jolt_fmt_ir::TextWidth::new(line_width),
                ..RenderOptions::default()
            },
        },
    );
    let expected = expected.to_owned() + "\n";

    assert_eq!(
        result.status,
        JavaFormatStatus::Formatted,
        "{source}\n{result:#?}"
    );
    assert_eq!(
        result.formatted_source.as_deref(),
        Some(expected.as_str()),
        "{source}"
    );
    assert!(result.diagnostics.is_empty(), "{source}");
}

fn assert_formats_successfully(source: &str) {
    let result = format_java_source(source);

    assert_eq!(
        result.status,
        JavaFormatStatus::Formatted,
        "{source}\n{result:#?}"
    );
    assert!(
        result.formatted_source.is_some(),
        "source should format: {source}"
    );
    assert!(result.diagnostics.is_empty(), "{source}\n{result:#?}");
}

#[cfg(test)]
fn assert_blocked_parser(source: &str) {
    let result = format_java_source(source);

    assert_eq!(result.status, JavaFormatStatus::Blocked);
    assert_eq!(result.formatted_source, None);
    assert!(!result.diagnostics.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.stage == DiagnosticStage::Parser)
    );
}

#[cfg(test)]
fn assert_blocked_formatter(source: &str) {
    assert_blocked_formatter_with_message(source, None);
}

#[cfg(test)]
fn assert_blocked_formatter_with_message(source: &str, expected_message: Option<&str>) {
    let result = format_java_source(source);

    assert_eq!(result.status, JavaFormatStatus::Blocked);
    assert_eq!(result.formatted_source, None);
    assert!(!result.diagnostics.is_empty());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.stage == DiagnosticStage::Formatter)
    );
    if let Some(expected_message) = expected_message {
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message == expected_message),
            "{source}\n{result:#?}"
        );
    }
}

#[test]
fn imports_preserve_source_order() {
    assert_formatted(
        "import z.Z; import a.A; import java.util.*; import module java.base; import module.foo.Bar; class A {}",
        "import z.Z;\nimport a.A;\nimport java.util.*;\nimport module java.base;\nimport module.foo.Bar;\n\nclass A {}",
    );
}

#[test]
fn module_declarations_format() {
    assert_formatted(
        "@Deprecated open module example.module { requires transitive static java.sql; exports example.api to friend.module, other.module; opens example.internal; uses example.Service; provides example.Service with example.ServiceImpl, other.ServiceImpl; }",
        "@Deprecated\nopen module example.module {\n  requires transitive static java.sql;\n  exports example.api to friend.module, other.module;\n  opens example.internal;\n  uses example.Service;\n  provides example.Service with example.ServiceImpl, other.ServiceImpl;\n}",
    );
}

#[test]
fn compact_compilation_unit_members_format() {
    assert_formatted(
        "import java.util.List; ; void main() { System.out.println(List.of()); } int value; class Helper {}",
        "import java.util.List;\n\n;\n\nvoid main() {\n  System.out.println(List.of());\n}\n\nint value;\n\nclass Helper {}",
    );
}

#[test]
fn compact_compilation_unit_member_comments_format() {
    assert_formatted(
        "// main\nvoid main() { return; } // trailing main\n// value\nint value; // trailing field",
        "// main\nvoid main() {\n  return;\n} // trailing main\n\n// value\nint value; // trailing field",
    );
}

#[test]
fn class_body_empty_declarations_format() {
    assert_formatted(
        "class A { ; int value; ; // trailing\n; }",
        "class A {\n  ;\n\n  int value;\n\n  ; // trailing\n\n  ;\n}",
    );
}

#[test]
fn method_and_constructor_signatures_format_structurally() {
    assert_formatted(
        "abstract class A { public <T, U> T pick(final T first, U second) throws Problem, java.io.IOException { return first; } private A(int count, String... names) throws Problem {} abstract void reset(int count) throws Problem; }",
        "abstract class A {\n  public <T, U> T pick(final T first, U second) throws Problem, java.io.IOException {\n    return first;\n  }\n\n  private A(int count, String... names) throws Problem {}\n\n  abstract void reset(int count) throws Problem;\n}",
    );
    assert_formatted(
        "class A { void a(@N T v, @A final String n, Object @N ... r) {} void legacy(int v[]) {} }",
        "class A {\n  void a(@N T v, @A final String n, Object @N ... r) {}\n\n  void legacy(int v[]) {}\n}",
    );
}

#[test]
fn constructor_invocations_format() {
    assert_formatted(
        "class A extends Base { A() { this(0); } <T> A(T value) { <T>super(value); } class Inner extends Base { Inner(A outer) { outer.super(0); } } }",
        "class A extends Base {\n  A() {\n    this(0);\n  }\n\n  <T> A(T value) {\n    <T>super(value);\n  }\n\n  class Inner extends Base {\n    Inner(A outer) {\n      outer.super(0);\n    }\n  }\n}",
    );
}

#[test]
fn class_headers_and_nested_classes_format_structurally() {
    assert_formatted(
        "class A<T, U> extends base.Parent implements First, second.Third permits One, two.Three { private static class Nested extends Parent implements Marker {} }",
        "class A<T, U> extends base.Parent implements First, second.Third permits One, two.Three {\n  private static class Nested extends Parent implements Marker {}\n}",
    );
}

#[test]
fn record_declarations_format_structurally() {
    assert_formatted(
        "record Point<T>(@Deprecated int x, String @Marker ... labels) implements Named { Point { labels = labels.clone(); } public String name() { return labels[0]; } record Nested(int value) {} } interface Named {}",
        "record Point<T>(@Deprecated int x, String @Marker ... labels) implements Named {\n  Point {\n    labels = labels.clone();\n  }\n\n  public String name() {\n    return labels[0];\n  }\n\n  record Nested(int value) {}\n}\n\ninterface Named {}",
    );
}

#[test]
fn interface_declarations_format_structurally() {
    assert_formatted(
        "interface Api<T> extends Parent, second.Other { int VALUE = 1; void call(); class Nested {} interface Child {} ; }",
        "interface Api<T> extends Parent, second.Other {\n  int VALUE = 1;\n\n  void call();\n\n  class Nested {}\n\n  interface Child {}\n\n  ;\n}",
    );
}

#[test]
fn enum_declarations_format_structurally() {
    assert_formatted(
        "enum Empty {} enum Op implements Marker { @A ONE, TWO(1 + helper(\"x\"), new Box()), SPECIAL { void run() {} }; int code; Op(int code) { this.code = code; } enum Nested { VALUE } }",
        "enum Empty {}\n\nenum Op implements Marker {\n  @A ONE,\n  TWO(1 + helper(\"x\"), new Box()),\n  SPECIAL {\n    void run() {}\n  }\n  ;\n  int code;\n  Op(int code) {\n    this.code = code;\n  }\n  enum Nested {\n    VALUE\n  }\n}",
    );
    assert_formatted("enum E { A, }", "enum E {\n  A,\n}");
}

#[test]
fn annotation_interface_declarations_format_structurally() {
    assert_formatted(
        "@Marker public @interface Contract { String value() default \"x\"; int answer() default 1 + 1; class Helper {} @interface Nested {} } interface Host { @interface Tag {} }",
        "@Marker\npublic @interface Contract {\n  String value() default \"x\";\n\n  int answer() default 1 + 1;\n\n  class Helper {}\n\n  @interface Nested {}\n}\n\ninterface Host {\n  @interface Tag {}\n}",
    );
    assert_formatted("@interface Empty {}", "@interface Empty {}");
}

#[test]
fn dangling_comments_inside_empty_interface_bodies_format() {
    assert_formatted(
        "interface Api {\n/** docs */\n// line\n}",
        "interface Api {\n  /** docs */\n  // line\n}",
    );
}

#[test]
fn declaration_marker_annotations_format_vertically() {
    assert_formatted(
        "@Pkg package com.example; @Type public class A { @Field private String value; @Method public String name() { return value; } @Ctor A() {} @Nested static class Nested {} }",
        "@Pkg\npackage com.example;\n\n@Type\npublic class A {\n  @Field\n  private String value;\n\n  @Method\n  public String name() {\n    return value;\n  }\n\n  @Ctor\n  A() {}\n\n  @Nested\n  static class Nested {}\n}",
    );
}

#[test]
fn declaration_annotation_arguments_format_structurally() {
    assert_formatted(
        "@Single(\"type\") @Normal(first = 1, second=value + 2) class A { @SuppressWarnings(\"unchecked\") String value; }",
        "@Single(\"type\")\n@Normal(first = 1, second = value + 2)\nclass A {\n  @SuppressWarnings(\"unchecked\")\n  String value;\n}",
    );
}

#[test]
fn type_use_annotations_in_simple_types_format_structurally() {
    assert_formatted(
        "class A { java.lang.@Anno String value; void m() { java.lang.@Anno String local; } }",
        "class A {\n  java.lang.@Anno String value;\n\n  void m() {\n    java.lang.@Anno String local;\n  }\n}",
    );
}

#[test]
fn generic_and_array_types_format_structurally() {
    assert_formatted(
        "class A<@Anno T extends java.lang.Object & java.io.Serializable> extends java.util.List<String> { java.util.Map<String, ? extends Number> upper; java.util.List<? super T>[] lower; T[][] matrix; java.util.List<String> names; String[] names() {} void m(java.util.List<String> input) { java.util.List<String> local = new java.util.ArrayList<String>(); } }",
        "class A<@Anno T extends java.lang.Object & java.io.Serializable> extends java.util.List<String> {\n  java.util.Map<String, ? extends Number> upper;\n  java.util.List<? super T>[] lower;\n  T[][] matrix;\n  java.util.List<String> names;\n\n  String[] names() {}\n\n  void m(java.util.List<String> input) {\n    java.util.List<String> local = new java.util.ArrayList<String>();\n  }\n}",
    );
}

#[test]
fn non_empty_method_and_constructor_blocks_format_in_source_order() {
    assert_formatted(
        "class A { A() { int local; { return; } } int one() { return 1; } Object self() { return this; } Object parent() { return super; } void done() { return; } }",
        "class A {\n  A() {\n    int local;\n    {\n      return;\n    }\n  }\n\n  int one() {\n    return 1;\n  }\n\n  Object self() {\n    return this;\n  }\n\n  Object parent() {\n    return super;\n  }\n\n  void done() {\n    return;\n  }\n}",
    );
}

#[test]
fn local_variable_types_and_throw_statements_format_structurally() {
    assert_formatted(
        "class A { void fail() { java.lang.Exception ex; var var = ex; final var copy = var; throw ex; } }",
        "class A {\n  void fail() {\n    java.lang.Exception ex;\n    var var = ex;\n    final var copy = var;\n    throw ex;\n  }\n}",
    );
}

#[test]
fn local_variable_annotations_and_declarator_dimensions_format() {
    assert_formatted(
        "class A { void m(Object values) { @Marker final Object annotated; @Marker Object plain; String legacy[] = names; int numbers[] = {1, 2}; var _ = values; } }",
        "class A {\n  void m(Object values) {\n    @Marker\n    final Object annotated;\n    @Marker\n    Object plain;\n    String legacy[] = names;\n    int numbers[] = {\n      1, 2\n    };\n    var _ = values;\n  }\n}",
    );
}

#[test]
fn field_and_local_initializers_format_supported_expressions() {
    assert_formatted(
        "class A { int value = 1; Object output = System.out; int total = a + b * c; int grouped = (a + b) * -c; int negative = - -1; int positive = + +1; int choice = flag ? one : two; int first, second = 2; void m() { int local = (value + 1), other; } int sum() { return a + b * c; } }",
        "class A {\n  int value = 1;\n  Object output = System.out;\n  int total = a + b * c;\n  int grouped = (a + b) * -c;\n  int negative = - -1;\n  int positive = + +1;\n  int choice = flag ? one : two;\n  int first, second = 2;\n\n  void m() {\n    int local = (value + 1), other;\n  }\n\n  int sum() {\n    return a + b * c;\n  }\n}",
    );
    assert_formatted(
        "class A { String text = \"\"\"\n  alpha\n    beta\n  \"\"\"; String inline = \"\"\"\n  omega\"\"\"; }",
        "class A {\n  String text =\n      \"\"\"\n      alpha\n        beta\n      \"\"\";\n  String inline =\n      \"\"\"\n      omega\\\n      \"\"\";\n}",
    );
    assert_formatted(
        r#"class A { String text = """
  slash\\""";
}"#,
        r#"class A {
  String text =
      """
  slash\\""";
}"#,
    );
    assert_formatted(
        "class A { String text = \"\"\"\n    value\n  \"\"\"; }",
        "class A {\n  String text =\n      \"\"\"\n        value\n      \"\"\";\n}",
    );
}

#[test]
fn instanceof_expressions_format() {
    assert_formatted(
        "class A { boolean m(Object value) { return value instanceof java.util.List<?> || value instanceof final String text; } }",
        "class A {\n  boolean m(Object value) {\n    return value instanceof java.util.List<?> || value instanceof final String text;\n  }\n}",
    );
}

#[test]
fn class_literal_expressions_format() {
    assert_formatted(
        "class A { Object type = String.class; Object qualified = java.lang.String.class; Object primitive = int.class; Object none = void.class; Object array = String[][].class; Object primitiveArray = int[].class; Object m() { return String.class; } void call() { use(String.class, int.class); } }",
        "class A {\n  Object type = String.class;\n  Object qualified = java.lang.String.class;\n  Object primitive = int.class;\n  Object none = void.class;\n  Object array = String[][].class;\n  Object primitiveArray = int[].class;\n\n  Object m() {\n    return String.class;\n  }\n\n  void call() {\n    use(String.class, int.class);\n  }\n}",
    );
}

#[test]
fn object_creation_expressions_format() {
    assert_formatted(
        "class A { Object value = new Object(); Object qualified = new java.lang.Object(); Object withArgs = new Pair(first, second); Object m() { return new Object(); } void call() { new Object(); use(new Object()); } }",
        "class A {\n  Object value = new Object();\n  Object qualified = new java.lang.Object();\n  Object withArgs = new Pair(first, second);\n\n  Object m() {\n    return new Object();\n  }\n\n  void call() {\n    new Object();\n    use(new Object());\n  }\n}",
    );
}

#[test]
fn object_creation_variants_format() {
    assert_formatted(
        "class A { Runnable r = new Runnable() { public void run() {} }; Object inner = outer.new Inner(); Object typed = new <String> Box(value); }",
        "class A {\n  Runnable r =\n      new Runnable() {\n        public void run() {}\n      };\n  Object inner = outer.new Inner();\n  Object typed = new <String>Box(value);\n}",
    );
}

#[test]
fn casts_arrays_and_switch_expressions_format() {
    assert_formatted(
        "class A { Object[] values = new Object[] { one, (String) two, new String @A [] { three, }, }; int[] sized = new int[count]; Object choice(int x) { values[0] = values[count - 1]; return switch (x) { case 1 -> new Object(); default -> (Object) fallback; }; } }",
        "class A {\n  Object[] values =\n      new Object[] {\n        one, (String) two, new String @A [] {\n          three,\n        },\n      };\n  int[] sized = new int[count];\n\n  Object choice(int x) {\n    values[0] = values[count - 1];\n    return switch (x) {\n      case 1 -> new Object();\n      default -> (Object) fallback;\n    };\n  }\n}",
    );
}

#[test]
fn lambda_expressions_format() {
    assert_formatted(
        "class A { void m() { java.util.function.Function<String, String> trim = (String s) -> s.trim(); java.util.function.IntUnaryOperator inc = x -> x + 1; java.util.function.BinaryOperator<Integer> add = (x, y) -> x + y; java.util.function.Supplier<Integer> batch = () -> DEFAULT_BATCH_SIZE; java.util.function.BiPredicate<Object, Object> named = (final var dir, final var name) -> true; java.util.function.Function<Object, Object> identity = (var value) -> value; java.util.function.IntFunction<Integer> zero = _ -> 0; java.util.function.Function<String[], Integer> lengths = (String... values) -> values.length; Runnable run = () -> { call(); }; java.util.function.Consumer<String> consume = value -> { use(value); }; } }",
        "class A {\n  void m() {\n    java.util.function.Function<String, String> trim = (String s) -> s.trim();\n    java.util.function.IntUnaryOperator inc = x -> x + 1;\n    java.util.function.BinaryOperator<Integer> add = (x, y) -> x + y;\n    java.util.function.Supplier<Integer> batch = () -> DEFAULT_BATCH_SIZE;\n    java.util.function.BiPredicate<Object, Object> named =\n        (final var dir, final var name) -> true;\n    java.util.function.Function<Object, Object> identity = (var value) -> value;\n    java.util.function.IntFunction<Integer> zero = _ -> 0;\n    java.util.function.Function<String[], Integer> lengths = (String... values) -> values.length;\n    Runnable run =\n        () -> {\n          call();\n        };\n    java.util.function.Consumer<String> consume =\n        value -> {\n          use(value);\n        };\n  }\n}",
    );
}

#[test]
fn method_reference_expressions_format() {
    assert_formatted(
        "class A { void m() { Object a = target::name; Object b = Type::name; Object c = Type::new; Object d = String[]::new; Object e = this::<String>id; Object f = ArrayList<String>::new; Object g = java.util.ArrayList<String>::new; call(values.stream().map(Baz::getId)); } }",
        "class A {\n  void m() {\n    Object a = target::name;\n    Object b = Type::name;\n    Object c = Type::new;\n    Object d = String[]::new;\n    Object e = this::<String>id;\n    Object f = ArrayList<String>::new;\n    Object g = java.util.ArrayList<String>::new;\n    call(values.stream().map(Baz::getId));\n  }\n}",
    );
}

#[test]
fn initializer_blocks_format_as_class_body_members() {
    assert_formatted(
        "class A { static { int ready; } { call(); } }",
        "class A {\n  static {\n    int ready;\n  }\n\n  {\n    call();\n  }\n}",
    );
}

#[test]
fn expression_statements_format_supported_calls_assignments_and_updates() {
    assert_formatted(
        "class A { void m() { call(); target.call(1, this.value); System.out.println((value)); builder.first().second(value); this.value = value + 1; value += -delta; value++; ++value; } }",
        "class A {\n  void m() {\n    call();\n    target.call(1, this.value);\n    System.out.println((value));\n    builder.first().second(value);\n    this.value = value + 1;\n    value += -delta;\n    value++;\n    ++value;\n  }\n}",
    );
}

#[test]
fn generic_qualified_method_invocations_format() {
    assert_formatted(
        "class A { void m() { this.<String>generic(\"value\"); target.<String>generic(\"value\"); Type.<String>staticGeneric(\"value\"); super.<String>baseGeneric(\"value\"); new Builder<String>().add(\"value\").build(); } }",
        "class A {\n  void m() {\n    this.<String>generic(\"value\");\n    target.<String>generic(\"value\");\n    Type.<String>staticGeneric(\"value\");\n    super.<String>baseGeneric(\"value\");\n    new Builder<String>().add(\"value\").build();\n  }\n}",
    );
}

#[test]
fn selector_receivers_format_general_expressions() {
    assert_formatted(
        "class A { void m(Object value, boolean flag, Object one, Object two) { (flag ? one : two).toString(); ((String) value).trim(); } }",
        "class A {\n  void m(Object value, boolean flag, Object one, Object two) {\n    (flag ? one : two).toString();\n    ((String) value).trim();\n  }\n}",
    );
}

#[test]
fn argument_parameter_comments_format_inline() {
    assert_formatted(
        "class A { void m() { call(/*a=*/ 1, /* b */ value, false /* off */); } }",
        "class A {\n  void m() {\n    call(/*a=*/ 1, /* b */ value, false /* off */);\n  }\n}",
    );
}

#[test]
fn list_helpers_own_leading_item_comments() {
    for source in [
        "class A { void m() { call(\n// argument\nvalue); } }",
        "class A { void clear(\n// parameter\n@Anno int value) {} }",
        "class A<\n// type parameter\nT> {}",
        "class A { java.util.List<\n// type argument\nString> value; }",
        "class A { java.util.List<\n// type argument\n@Anno String> value; }",
        "record A(\n// component\nint value) {}",
        "class A { void m() throws\n// problem\nException {} }",
    ] {
        assert_formats_successfully(source);
    }
}

#[test]
fn narrow_width_wraps_existing_argument_lists() {
    assert_formatted_with_width(
        "class A { void m() { call(alpha, beta, gamma); } }",
        "class A {\n  void m() {\n    call(\n        alpha, beta,\n        gamma);\n  }\n}",
        20,
    );
}

#[test]
fn narrow_width_wraps_method_signature_parameters() {
    assert_formatted_with_width(
        "class A { void combine(int alpha, int beta, int gamma) throws FirstProblem, SecondProblem {} }",
        "class A {\n  void combine(\n      int alpha,\n      int beta,\n      int gamma)\n      throws FirstProblem,\n          SecondProblem {\n  }\n}",
        20,
    );
}

#[test]
fn narrow_width_wraps_type_parameter_and_argument_lists() {
    assert_formatted_with_width(
        "class A<Alpha, Beta, Gamma> { java.util.Map<Alpha, Beta> value; }",
        "class A<\n        Alpha,\n        Beta,\n        Gamma> {\n  java.util.Map<\n          Alpha,\n          Beta> value;\n}",
        20,
    );
}

#[test]
fn narrow_width_wraps_existing_variable_declarations() {
    assert_formatted_with_width(
        "class A { int total = alpha + beta + gamma; void m() { final int local = alpha + beta + gamma; } }",
        "class A {\n  int total =\n      alpha + beta\n          + gamma;\n\n  void m() {\n    final int local =\n        alpha + beta\n            + gamma;\n  }\n}",
        20,
    );
}

#[test]
fn narrow_width_wraps_existing_assignments_and_binary_expressions() {
    assert_formatted_with_width(
        "class A { void m() { target.value = alpha + beta + gamma; } }",
        "class A {\n  void m() {\n    target.value =\n        alpha + beta\n            + gamma;\n  }\n}",
        20,
    );
}

#[test]
fn narrow_width_wraps_existing_selector_chains() {
    assert_formatted_with_width(
        "class A { void m() { builder.first().second(value).third(); } }",
        "class A {\n  void m() {\n    builder.first()\n        .second(\n            value)\n        .third();\n  }\n}",
        20,
    );
}

#[test]
fn invalid_java_blocks_and_forwards_parser_diagnostics() {
    assert_blocked_parser("class A {");
}

#[test]
fn ignored_trivia_remains_unowned_formatter_debt() {
    assert_blocked_formatter("class A {}\u{001A}");
}

#[test]
fn leading_comments_before_compilation_unit_declarations_format() {
    assert_formatted(
        "// package\npackage com.example;\n// import\nimport java.util.List;\n// type\nclass A {}",
        "// package\npackage com.example;\n\n// import\nimport java.util.List;\n\n// type\nclass A {}",
    );
}

#[test]
fn leading_comments_before_members_and_block_statements_format() {
    assert_formatted(
        "class A {\n// field\nint value;\n/** method */\nvoid clear() {\n// local\nint local = 1;\n// call\ncall();\n{\n// nested\nreturn;\n}\n}\n}",
        "class A {\n  // field\n  int value;\n\n  /** method */\n  void clear() {\n    // local\n    int local = 1;\n    // call\n    call();\n    {\n      // nested\n      return;\n    }\n  }\n}",
    );
}

#[test]
fn leading_javadocs_before_class_and_method_format() {
    assert_formatted(
        "/** class docs */\nclass A {\n/** method docs */\nvoid clear() {} }",
        "/** class docs */\nclass A {\n  /** method docs */\n  void clear() {}\n}",
    );
}

#[test]
fn multiline_leading_block_comments_and_javadocs_format() {
    assert_formatted(
        "/*\n * class docs\n */\nclass A {\n/**\n * field docs\n */\nint value;\nvoid clear() {\n/*\n * local docs\n */\nreturn;\n}\n}",
        "/*\n * class docs\n */\nclass A {\n  /** field docs */\n  int value;\n\n  void clear() {\n    /*\n     * local docs\n     */\n    return;\n  }\n}",
    );
}

#[test]
fn single_body_multiline_javadocs_collapse_to_one_line() {
    assert_formatted(
        "class A {\n  /**\n   * field docs\n   */\n  int value;\n}",
        "class A {\n  /** field docs */\n  int value;\n}",
    );
}

#[test]
fn dangling_comments_inside_empty_class_bodies_format() {
    assert_formatted(
        "class A {\n/*\n * block\n */\n/** docs */\n// line\n}",
        "class A {\n  /*\n   * block\n   */\n  /** docs */\n  // line\n}",
    );
}

#[test]
fn dangling_comments_inside_empty_blocks_format() {
    assert_formatted(
        "class A { void clear() {\n// line\n} A() {\n/**\n * constructor\n */\n} }",
        "class A {\n  void clear() {\n    // line\n  }\n\n  A() {\n    /** constructor */\n  }\n}",
    );
}

#[test]
fn trailing_line_comments_after_declarations_and_statements_format() {
    assert_formatted(
        "class A { int value = 1; // field\nint one() { call(); // call\nreturn 1; // answer\n} }",
        "class A {\n  int value = 1; // field\n\n  int one() {\n    call(); // call\n    return 1; // answer\n  }\n}",
    );
}

#[test]
fn comment_forms_format() {
    for source in [
        "class A { // dangling\n}",
        "class A { void clear() { // dangling\n} }",
        "class A { void clear(\n// parameter\nint value) {} }",
        "class A { abstract void clear(\n// parameter\nint value); }",
        "class A { void clear() throws\n// throws\nException {} }",
        "class A { /* body */ }",
        "class A { void clear() { /* body */ } }",
    ] {
        assert_formats_successfully(source);
    }
}

#[test]
fn annotation_comment_forms_format() {
    assert_blocked_formatter("@ /* inside */ Anno class A {}");
}

#[test]
fn annotation_argument_comments_remain_unowned_formatter_debt() {
    assert_blocked_formatter("class A { @Anno(\n// value\n1) int value; }");
}

#[test]
fn header_and_annotation_between_comments_remain_unowned_formatter_debt() {
    for source in [
        "class A // header\n{}",
        "class A\n// header\nextends B {}",
        "class A { int /* inline */ value; }",
        "class A { void /* inline */ clear() {} }",
        "class A { void clear() { if (ready)\n// branch\nreturn; call(); } }",
        "class A { void clear() { if (ready) { return; }\n// else\nelse return; } }",
        "@Anno /* between */ class A {}",
        "@First /* between */ @Second class A {}",
    ] {
        assert_blocked_formatter(source);
    }
}

#[test]
fn boundary_comment_guards_report_domain_messages() {
    for (source, message) in [
        (
            "@ /* inside */ Anno class A {}",
            "Java formatter does not support comments inside declaration annotations yet",
        ),
        (
            "class A { java.util.List<@A /* gap */ String> value; }",
            "Java formatter does not support comments between type-use annotations and types yet",
        ),
        (
            "class A // header\n{}",
            "Java formatter does not support comments inside class headers yet",
        ),
        (
            "class A { void clear() { if (ready)\n// branch\nreturn; } }",
            "Java formatter does not support comments before if statement bodies yet",
        ),
    ] {
        assert_blocked_formatter_with_message(source, Some(message));
    }
}

#[test]
fn simple_statement_forms_format() {
    assert_formatted(
        "class A { void m() { ; if (ready) { return; } else if (other) break label; else continue; } }",
        "class A {\n  void m() {\n    ;\n    if (ready) {\n      return;\n    } else if (other)\n      break label;\n    else\n      continue;\n  }\n}",
    );
}

#[test]
fn label_and_assert_statements_format() {
    assert_formatted(
        "class A { void m() { label: return; assert ready; assert ready : message; } }",
        "class A {\n  void m() {\n    label:\n    return;\n    assert ready;\n    assert ready : message;\n  }\n}",
    );
}

#[test]
fn switch_statements_format_constant_groups_and_rules() {
    assert_formatted(
        "class A { void m(int x) { switch (x) { case 1: call(); break; case 2: case 3: return; default: throw problem; } switch (x) { case 1, -2, NAME -> value++; case null, default -> { break; } } } }",
        "class A {\n  void m(int x) {\n    switch (x) {\n      case 1:\n        call();\n        break;\n      case 2:\n      case 3:\n        return;\n      default:\n        throw problem;\n    }\n    switch (x) {\n      case 1, -2, NAME -> value++;\n      case null, default -> {\n        break;\n      }\n    }\n  }\n}",
    );
}

#[test]
fn switch_pattern_labels_format() {
    assert_formatted(
        "class A { void m(Object value) { switch (value) { case String text when text.isEmpty() -> call(); case Integer _ : default: fallback(); } } }",
        "class A {\n  void m(Object value) {\n    switch (value) {\n      case String text when text.isEmpty() -> call();\n      case Integer _:\n      default:\n        fallback();\n    }\n  }\n}",
    );
}

#[test]
fn simple_loop_statements_format() {
    assert_formatted(
        "class A { void m() { while (ready) return; while (again) { call(); } do continue; while (ready); do { call(); } while (again); for (;;) return; for (int i = 0; i < limit; i++) { call(i); } for (value = 0, other = 1; value < limit; value++, other++) call(value); for (String value : values) call(value); } }",
        "class A {\n  void m() {\n    while (ready)\n      return;\n    while (again) {\n      call();\n    }\n    do\n      continue;\n    while (ready);\n    do {\n      call();\n    } while (again);\n    for (;;)\n      return;\n    for (int i = 0; i < limit; i++) {\n      call(i);\n    }\n    for (value = 0, other = 1; value < limit; value++, other++)\n      call(value);\n    for (String value : values)\n      call(value);\n  }\n}",
    );
}

#[test]
fn synchronized_statements_format() {
    assert_formatted(
        "class A { void m() { synchronized (lock) { call(); } synchronized (this.value) {} } }",
        "class A {\n  void m() {\n    synchronized (lock) {\n      call();\n    }\n    synchronized (this.value) {}\n  }\n}",
    );
}

#[test]
fn simple_try_catch_finally_statements_format() {
    assert_formatted(
        "class A { void m() { try { call(); } catch (final Exception ex) { handle(ex); } finally { cleanup(); } } }",
        "class A {\n  void m() {\n    try {\n      call();\n    } catch (final Exception ex) {\n      handle(ex);\n    } finally {\n      cleanup();\n    }\n  }\n}",
    );
}

#[test]
fn simple_try_catch_statements_format() {
    assert_formatted(
        "class A { void m() { try { call(); } catch (Exception ex) { handle(ex); } } }",
        "class A {\n  void m() {\n    try {\n      call();\n    } catch (Exception ex) {\n      handle(ex);\n    }\n  }\n}",
    );
}

#[test]
fn catch_parameter_variants_format() {
    assert_formatted(
        "class A { void m() { try { call(); } catch (@Critical final java.io.IOException | RuntimeException ex) { handle(ex); } } }",
        "class A {\n  void m() {\n    try {\n      call();\n    } catch (@Critical final java.io.IOException | RuntimeException ex) {\n      handle(ex);\n    }\n  }\n}",
    );
}

#[test]
fn simple_try_finally_statements_format() {
    assert_formatted(
        "class A { void m() { try { return; } finally {} } }",
        "class A {\n  void m() {\n    try {\n      return;\n    } finally {}\n  }\n}",
    );
}

#[test]
fn try_with_resources_statements_format() {
    assert_formatted(
        "class A { void m(Resource existing) { try (final Resource r = open(); existing;) { use(r); } catch (Exception ex) { handle(ex); } finally { cleanup(); } try (this.existing) {} } }",
        "class A {\n  void m(Resource existing) {\n    try (final Resource r = open(); existing) {\n      use(r);\n    } catch (Exception ex) {\n      handle(ex);\n    } finally {\n      cleanup();\n    }\n    try (this.existing) {}\n  }\n}",
    );
}
