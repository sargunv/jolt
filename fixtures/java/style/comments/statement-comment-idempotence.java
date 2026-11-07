class StatementCommentIdempotence {
  void forHeaders() {
    for ( /* an empty initializer whose long comment does not fit on the line */ ;;) {
      break;
    }
  }

  void switchGroups(int k) {
    switch (k) {
      case 1:
        run();
      /* the comment of a removed semicolon */
      ;
      case 2:
        run();
      // the last group's removed semicolon
      ;
    }
  }

  void switchRules(int k) {
    switch (k) {
      case 1 -> /* a */
      // b
      {}
      case 2 ->
      // c
      throw new IllegalStateException();
      default -> {}
    }
  }
}
