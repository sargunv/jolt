class CommentsAfterKeywordsAndArrows<T extends Comparable<T> & // bound
    Runnable, U extends Object &
    // own line before the bound
    Cloneable> {
  int keywords(int x) {
    if (x > 0) {
      return // after return
          x + 1;
    }
    if (x < 0) {
      return
          // own line before the value
          x - 1;
    }
    if (x == 1) {
      throw // after throw
          new IllegalStateException();
    }
    if (x == 2) {
      throw
          // own line before the exception
          new IllegalStateException();
    }
    if (x == 3) {
      return /* inline */ x;
    }
    return switch (x) {
      case 4 -> {
        yield // after yield
            x;
      }
      case 5 -> {
        yield
            // own line before the value
            x;
      }
      default -> x;
    };
  }

  void rules(int x) {
    switch (x) {
      case 1 -> // after the arrow
          throw new IllegalStateException();
      case 2 ->
          // own line before the throw
          throw new IllegalStateException();
      default -> {}
    }
  }

  void lambdas() {
    run(() -> // after the arrow
        foo());
    run(() ->
        // own line before the body
        foo());
  }

  void catches() {
    try {
      foo();
    } catch (IllegalStateException | // after the bar
        IllegalArgumentException e) {
      foo();
    }
  }
}
