class StatementTrivia {
  void run(int value) {
    if ( /* JOLT-TRIVIA:if-open */ value > 0 /* JOLT-TRIVIA:if-condition */) /* JOLT-TRIVIA:if-before-body */ {
      return; // JOLT-TRIVIA:return-tail
    } else /* JOLT-TRIVIA:else-tail */ if (value == 0) {
      throw new IllegalStateException(); /* JOLT-TRIVIA:throw-tail */
    }

    for (int i = 0 /* JOLT-TRIVIA:for-init */; i < value /* JOLT-TRIVIA:for-test */; i++ /* JOLT-TRIVIA:for-update */) {
      continue /* JOLT-TRIVIA:continue-before-semicolon */;
    }

    switch (value /* JOLT-TRIVIA:switch-selector */) {
      case 1 /* JOLT-TRIVIA:case-label */ -> /* JOLT-TRIVIA:case-arrow */ run(value - 1);
      default /* JOLT-TRIVIA:default-label */ -> {
        break; /* JOLT-TRIVIA:break-tail */
      }
    }
  }
}
