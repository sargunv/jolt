class SwitchOpeningBraceComment {
  void arrow(int value) {
    switch (value) { // JOLT-TRIVIA:arrow-open
      default -> done();
    }
  }

  void colon(int value) {
    switch (value) { // JOLT-TRIVIA:colon-open
      default:
        done();
    }
  }

  void done() {}
}
