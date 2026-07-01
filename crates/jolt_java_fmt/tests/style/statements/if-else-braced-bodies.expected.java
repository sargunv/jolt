class Example {
  void run() {
    if ( /* guard */ ready) {
      run();
    } else if (waiting) {
      pause();
    } else /* fallback */ {
      stop();
    }
  }
}
