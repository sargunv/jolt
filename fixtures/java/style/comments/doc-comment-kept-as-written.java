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

  void docCommentThatTheLayoutPutsOnItsOwnLine() {
    if (/** a doc comment  after an opener, which the line comment after it puts on its own line */
    // a line comment
    true) {}
  }

  /** a doc comment  before an empty declaration */ ;

  int afterTheEmptyDeclaration;

  /** @param
   * x @@ text, an escape that the line break before the tag's argument makes */
  void escapeAfterAnArgumentOnTheNextLine(int x) {}

  /**
   * 
   * <p>Text after a line of a form feed, which javadoc reads as text before the paragraph tag.
   */
  void formFeedLineBeforeAParagraph() {}

  /** <div> {@snippet : */
  void unfinishedSnippetBeforeTheEnd() {}

  /** {@snippet :
   * {@code a  b} after a form feed that makes the snippet erroneous */
  void formFeedAfterASnippetColon() {}

  void docCommentAfterARemovedEmptyStatement() {
    ; /** a doc comment  that the removed empty statements leave on its own line */ ;
    int x;
  }

  /**
   *  A, which starts with an em space that javadoc would strip as indentation
   * B
   */
  void emSpaceBeforeTheFirstWord() {}

  /**
   * @return value, after a form feed that javadoc would strip as indentation
   * more
   */
  int formFeedBeforeAnAtWithMoreText() {
    return 0;
  }

  ; /** a doc comment  of a removed semicolon, before a line comment */ // a line comment

  /** An unfinished index term right before the end: {@index x*/
  void unfinishedIndexTermBeforeTheEnd() {}

  /** <!--
   * -->@return value, after a form feed that keeps it from being a block tag */
  int formFeedAfterAMultilineHtmlComment() {
    return 0;
  }

  class StarlessLineAfterOpenerText { /** {@code a
 b} */ int x; }

  void docCommentInAPromotedBody() {
    if (true)
    /** a doc comment  before a line comment in a body that gets braces */ // a line comment
    run();
  }

  /** A brace right before the end: {*/
  void braceBeforeTheEnd() {}
}
