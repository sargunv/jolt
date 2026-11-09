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

  /**
   * Summary.
   *
   * @param value </p> the separator after a tag's argument stays, with enough words to wrap the line
   * @throws Exception </p> without it, javac reads {@code Exception</p>} as the reference
   * @see Object </li> {@link Object </p> and a link's reference}
   */
  void separatorAfterAReference(int value) throws Exception {}

  /** Summary.<ul><li>item, part of the first sentence because no space follows the period</li></ul> */
  void listAfterAPeriod() {}

  /**
   * Intro<br>
   * @return value, which javadoc reads as text after the form feed
   */
  int formFeedBeforeAnAt() {
    return 0;
  }

  /**
   * {@link
   * @return value
   */
  int unfinishedLink() {
    return 0;
  }

  /** Summary.<pre>code</pre> More text after the block, with enough words to wrap the line past the width. */
  void preAfterAPeriod() {}

  /** Summary.<h2>Heading</h2> More text after the heading, with enough words to wrap the line past the width. */
  void headingAfterAPeriod() {}

  /** Summary.<table><tr><td>cell</td></tr></table> More text after the table, with enough words to wrap. */
  void tableAfterAPeriod() {}

  /** <ul><li>Item. </li></ul> After, where the first sentence ends at the item. */
  void sentenceEndBeforeAClosingItem() {}

  /**
   *
   * -----
   * next line
   */
  void rulerAfterABlankFirstLine() {}

  /** <ul><li>First.<li>Second.</ul> After, where the first sentence holds both items. */
  void sentenceAcrossUnclosedItems() {}

  /** <div>Summary.</div> After, where the first sentence runs past the closing block. */
  void sentenceEndBeforeAClosingBlock() {}

  /** @see "A book"
   * More text, which makes the tag erroneous, keeps its lines.
   */
  void erroneousTagOnTheOpenerLine() {}

  /**
   * Text.
   *
   * @see "A book"
   * More text, which makes the tag erroneous, keeps its lines.
   */
  void erroneousTagOnItsOwnLine() {}

  /** A<p>Second, where the first sentence runs past the paragraph tag at offset 1. */
  void paragraphAfterALoneFirstCharacter() {}

  /** An index tag with no description keeps its brace: {@index "a long term" }@return text */
  void indexTagWithoutADescription() {}

  /** A </p> Second, where the first sentence ends at the closing tag after a space. */
  void closingParagraphAfterALoneFirstCharacter() {}

  /** {@snippet :<div>text</div>} where the block after the erroneous colon stays on its line. */
  void blockAfterAnErroneousSnippet() {}

  /**
   *
   * A<p>Second, where a blank line before the first word puts the paragraph tag past offset 1.
   */
  void paragraphAfterALoneCharacterOnALaterLine() {}

  /**
   *
   * A </p> Second, where a blank line before the first word puts the closing tag past offset 1.
   */
  void closingParagraphAfterALoneCharacterOnALaterLine() {}

  /**
   * @return value, which a form feed keeps from being a block tag
   * @since 1
   */
  int formFeedBeforeTheFirstWord() {
    return 0;
  }

  /** @code */
  void inlineOnlyTagAsABlockTag() {}

  /** A long enough description of the snippet tag that follows: {@snippet : <table>text</table>} */
  void tableAfterAnErroneousSnippet() {}
}
