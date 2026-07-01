class Example {
  void run(Object value) {
    switch ( /* selector */ value) {
      case 1:
      case 2:
        handleNumber();
        break;
      case String s when s.isEmpty() -> handleEmpty(s);
      case String s -> {
        handleString(s);
      }
      default -> throw new IllegalArgumentException();
    }
  }

  void blockCase(int value) {
    switch (value) {
      case 1: {
        int x = 1;
        break;
      }
      case 2: {
        int y = 2;
        break;
      }
      default:
        handleDefault();
    }
  }

  int classify(Object value) {
    return switch (value) {
      case null, // null arm
        default -> 0;
      case Pair(
          int left, // left
          _
        ) -> left;
    };
  }
}
