enum EnumTrailingCommaBlockComment {
  FIRST,
  /* JOLT-TRIVIA:enum-comma-block */;
}

enum EnumTrailingCommaLineComment {
  FIRST,
  // JOLT-TRIVIA:enum-comma-line
  ;
}

enum EnumBodyTrailingCommaComment {
  FIRST {},
  /* JOLT-TRIVIA:enum-body-comma */;
}
