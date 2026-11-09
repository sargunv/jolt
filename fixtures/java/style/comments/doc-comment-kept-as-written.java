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

  /** A{@literal b}C, whose form feed is text javadoc keeps, with enough words to wrap the line. */
  void literalFormFeed() {}

  /** A{@link String {@code  label}}C, a tag inside a tag, with enough words to wrap the line past the width. */
  void nestedInlineTag() {}

  /**
   * Summary, with enough words to wrap the line past the width of the comment here.
   * \u0040return the value
   */
  int unicodeEscapedTag() {
    return 0;
  }

  /** <pre><span title="</pre>">first  second</span></pre> */
  void preCloserInAnAttribute() {}

  /**
   * Summary.
   * @étiquette value of a custom tag, with enough words to wrap the line past the width
   */
  void nonAsciiTagName() {}

  /** The value is {@value "% d" #NUMBER}, with enough words to wrap the line past the width of it. */
  void quotedValueFormat() {}

  /**
   * * first item, with enough words to wrap the line past the width of the comment here
   */
  void leadingStar() {}

  /** * @return value */
  int leadingStarOnOneLine() {
    return 0;
  }

  /** {@value bad!
   * {@code good}, where javadoc's reading depends on whether the value's reference is valid */
  void unclosedValueTag() {}
}
