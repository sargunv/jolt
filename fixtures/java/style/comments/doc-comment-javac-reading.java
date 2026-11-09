class DocCommentJavacReading {
  /**
   * A no-break space after the star is text to javadoc, so it stays where it is:
   * indented by a no-break space, with enough words to wrap the line past the width.
   */
  void noBreakSpace() {}

  /** {@docRoot}{@docRoot}{@docRoot}{@docRoot}{@docRoot}{@docRoot}{@docRoot}{@docRoot}{@docRoot} adjacent tags, with enough words to wrap the line past the width. */
  void adjacentTags() {}

  /// A Markdown paragraph with a <pre>
  /// keeps   its
  ///   whitespace</pre> element whole, with enough words to wrap the line past the width.
  void preInAMarkdownParagraph() {}

  /// Only the first sentence of a Markdown comment ends at a line that starts with
  /// -1, so that line keeps its line break.
  ///
  /// In a later paragraph, a line that starts with
  /// -2 joins the line before it.
  void firstSentence() {}

  /**
   * A <a title="unclosed attribute value, which javadoc reads as an erroneous <, with enough words to wrap the line.
   */
  void unclosedAttributeValue() {}
}
