class FormatterIgnoreCommentBeforeOn {
  // @formatter:off
  void   raw( ) { }
 // owned by the next member
  // @formatter:on
  static void formatted() {}
}

class FormatterIgnoreCommentBeforeOnAtBoundary {
  // @formatter:off
  void   raw( ) { }
 // owned by the close brace
  // @formatter:on
}

class FormatterIgnoreRepeatedOffBeforeOn {
  // @formatter:off
  void   raw( ) { }
  // @formatter:off
  // @formatter:on
  static void formatted() {}
}
