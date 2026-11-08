class LeadingCommentsBeforeContinuedNodes<T
    /* bound */ extends Comparable<T>> {
  int[] blockComment = new int[]
      /* initializer */ { 1, 2 };
  int[] lineComment = new int[]
      // initializer
      { 1, 2 };
  int[] array = new int /* type */ [
      /* open */ /* close */]
      /* initializer */ {
        1 /* comma */,
        2 /* close */
      } /* semicolon */;

  @interface Element {
    int value()
        /* default */ default 1;
  }

  void f(Object o) {
    try {
      g();
    }
    /* finally */ finally {
      h();
    }
    switch (o) {
      case Integer i
          /* guard */ when i > 0 -> {}
      default -> {}
    }
  }
}
