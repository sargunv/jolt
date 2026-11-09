class DocCommentKeptAsWritten {
  /***
   * A banner, kept   as   written.
   ***/
  void banner() {}

  /**
   ***************************
   * A ruler of stars,   kept.
   ***************************
   */
  void rulerOfStars() {}

  /**
   ** Double, as javadoc reads it: the run of stars goes.
   */
  void doubleStar() {}

  void inline() {
    foo(/** an inline doc comment, which shares its line with code, is kept as written */ 1);
  }

  // @formatter:off
  /**
   * The Javadoc of an ignored member,   kept   as written.
   */
  void ignored() {}
  // @formatter:on

  /**
   * A multi-line code tag: {@code
   * @Bean
   * Foo foo() {}
   * } and {@code <ul>} are inline tags, so nothing inside them is a block tag or a list.
   * Braces balance: {@code "/{year:\d{1,4}}"} and {@code ${...}}.
   */
  void inlineTagsFirst() {}

  /** A{@literal  b}C, where the second space is text javadoc keeps, with enough words to wrap the line. */
  void literalLeadingSpace() {}

  /**
   * A {@code
   * value} whose content starts after a line break, with enough words to wrap.
   */
  void codeAfterALineBreak() {}

  /** A {@code value } whose content ends in a space, with enough words to wrap the line. */
  void codeTrailingSpace() {}
}
