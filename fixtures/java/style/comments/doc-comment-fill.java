class DocCommentFill {
  /**
   * Resolve the given placeholder using the properties of this resolver, which may be long.
   * Short line.
   * Another short line that joins the paragraph above it.
   */
  void paragraph() {}

  /** A one-line comment that is much too long to fit on its line at this indentation. */
  void tooLongForOneLine() {}

  /**
   * Fits on one line.
   */
  void fitsOnOneLine() {}

  /**
   */
  void empty() {}

  /** @return the value */
  int onlyTag() { return 0; }

  /**
   * Text.
   * @param placeholder    the placeholder to resolve, which may contain
   *        nested placeholders
   * @param second the second value
   *
   * @return the value
   * @throws IllegalArgumentException if the placeholder is invalid or cannot be resolved here
   */
  int tags(String placeholder, String second) { return 0; }

  /**
   * A word that would start a block tag, such as the annotation @Bean, or a list item, such as -
   * or 2. here, never starts a line, so the pairs move to the next line together, as in Java 8.
   */
  void neverBreakIntoStructure() {}

  /**
   * See {@link org.springframework.core.env.PropertyResolver#resolvePlaceholders(String)} for
   * details, and {@link #resolve(String, int, long) the resolve method with several arguments}.
   * The reference {@link
   * Object#toString()} is joined, and {@code Map<String, List<Object>> values} stays together.
   */
  void inlineTags() {}

  /**
   * See <a href="https://example.com/a/very/long/path/that/does/not/fit/on/any/line/of/the/comment">the
   * page</a> and <a href="https://example.com">a link with spaces in its tag</a> for details.
   * Write &#42;/ for the end of a comment.
   */
  void atoms() {}

  class Nested {
    class Deeper {
      class Deepest {
        /**
         * At deep indentation the text fills to the line width minus the column, and a word too long for its line overflows: {@link java.util.concurrent.ConcurrentHashMap#computeIfAbsent(Object, java.util.function.Function)}.
         */
        void deep() {}
      }
    }
  }
}
