class DelimiterTrivia {
  void emptyBlock() { /* JOLT-TRIVIA:empty-block-open */ // JOLT-TRIVIA:empty-block-line
    /* JOLT-TRIVIA:empty-block-close */
  }

  void emptyArguments() {
    call( /* JOLT-TRIVIA:empty-call-open */
      /* JOLT-TRIVIA:empty-call-close */
    );
  }

  void mixedDelimiters() {
    int[] values = new int[] { /* JOLT-TRIVIA:array-open */ 1, /* JOLT-TRIVIA:array-middle */ 2 /* JOLT-TRIVIA:array-tail */ };
    Object nested = (( /* JOLT-TRIVIA:paren-open */ values /* JOLT-TRIVIA:paren-close */ ));
  }

  void call(Object... args) {}
}
