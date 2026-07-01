class Example {
  void run(java.util.List<String> values) {
    retry /* label */:
    for ( /* each */ String value : values) {
      if (skip(value)) {
        continue retry /* again */;
      }
      if (stop(value)) {
        break retry /* target */;
      }
      process(value);
    }
    while ( /* while */ ready()) {
    }
    do {
      processNext();
    } while ( /* done */ hasNext());
    for ( /* forever */ ;;) {
    }
    synchronized ( /* lock */ this) {
      check();
    }
  }
}
