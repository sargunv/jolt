enum class CommentedEntrySeparator {
  FIRST, SECOND
  /* before members */;
  fun member() {}
}

enum class LineCommentedEntrySeparator {
  FIRST, SECOND
  // before members
  ;
  fun member() {}
}

enum class TrailingLineCommentedEntrySeparator {
  FIRST, SECOND // before members
  ;
  fun member() {}
}

enum class TrailingBlockCommentedEntrySeparator {
  FIRST, SECOND /* before members */
  ;
  fun member() {}
}

enum class IgnoredEntryBeforeSeparator {
  FIRST,
  // @formatter:off
  SECOND
  // @formatter:on
  ;
  fun member() {}
}
