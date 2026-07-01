class Example {
  void run() {
    for (int i = 0; i < 10; i++) {
      run(i);
    }
    for (i = 0, j = 0; i < count; i++, j++) {
      run(i, j);
    }
    for (; ready(); tick()) {
      run();
    }
    for (
      int index = startIndex, limit = computeLimit(input);
      index < limit && shouldContinue(index);
      index++, processed++
    ) {
      run(index);
    }
  }
}
