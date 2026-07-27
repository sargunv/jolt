class RemovedStraySemicolonCommentIdempotence {
  int field;
  /* JOLT-TRIVIA:stray-semicolon */ ;
  static {}

  void method() {}
  /*
   * JOLT-TRIVIA:multiline-stray-semicolon
   */ ;
  int following;
}
