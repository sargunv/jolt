class FormatterIgnoreLeadingCommentBeforeTrailingMarker {
  // JOLT-TRIVIA:member-leading
  int value = 1; // @formatter:off
  // @formatter:on
}

// JOLT-TRIVIA:top-level-leading
class TopLevelIgnored {} // @formatter:off
// @formatter:on
