class Example {
  void run(Object value) {
    switch (value) {
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

  int classify(Object value) {
    return switch (value) {
      case null, default -> 0;
      case Pair(int left, int right) -> left + right;
    };
  }
}
