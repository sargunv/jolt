import java.io.InputStream;

@interface A {}

class LeadingCommentOwnership {
  /** Before a structured modifier that precedes an annotation. */
  public @A static class DocumentedModifierFirst {}

  @A
  // Between an annotation and a field type.
  String annotatedField;

  @A
  // Between an annotation and a method return type.
  void annotatedMethod() {}

  @A
  // Between an annotation and a modifier token.
  public String modifiedField;

  public
  // Between a modifier token and an annotation.
  @A static class Nested {}

  public // Trailing modifier comment.
  @A static class Trailing {}

  /** Generic method documentation. */
  <T> @A T method(T value) {
    return value;
  }

  /** Generic constructor documentation. */
  <T> LeadingCommentOwnership(T value) {}

  void declarations(Iterable<String> values, InputStream input, Object candidate)
      throws Exception {
    int before = 0;
    // Local variable documentation.
    @A String local = "";

    for (
        // Enhanced-for variable documentation.
        @A String value : values) {}

    try (
        // Resource variable documentation.
        final InputStream resource = input) {}

    if (candidate
        instanceof
        // Type pattern documentation.
        final String text) {}
  }

}

@A
// Between an annotation and a type keyword.
class AnnotatedType {}

record Compact(int value) {
  /** Compact constructor documentation. */
  @A Compact {}
}

@interface Elements {
  /** Annotated element documentation. */
  @A String value();

  /** Modified element documentation. */
  public String explicit();
}

enum Values {
  /** Annotated constant documentation. */
  @Deprecated A,
  B,
}
