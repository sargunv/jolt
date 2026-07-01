class Example {
  void run(java.util.List<String> values) {
    retry /* label */:
    for (String value : values) {
      if (skip(value)) {
        continue retry /* again */;
      }
      if (stop(value)) {
        break retry /* target */;
      }
      process(value);
    }
    while (ready()) {
    }
    do {
      processNext();
    } while (hasNext());
    for (;;) {
    }
    synchronized (this) {
      check();
    }
  }
}
