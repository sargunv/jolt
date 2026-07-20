class RecoveredStatements {
  void enhancedFor(int[] values) {
    for (int value = /* enhanced-initializer */ 0 : values) {}
    for (int first, /* enhanced-extra */ second : values) {}
    for (int value, /* enhanced-trailing */ : values) {}
  }

  void resources(AutoCloseable existing) {
    try (AutoCloseable missing /* resource-initializer */) {}
    try (AutoCloseable first = open(), /* resource-extra */ second = open()) {}
    try (AutoCloseable first = open(), /* resource-trailing */) {}
    try (/* resource-missing-access */) {}
    try (existing /* resource-junk */ @) {}
    try (call() /* resource-invalid-access */) {}
    try (var malformed = /* resource-trailing */ ;) {}
    try {} catch (Exception exception /* catch-dimension */ []) {}
  }

  void switches(int value) {
    switch (value) {
      /* switch-unowned */ @;
      case 1 when ready() -> use();
      case 2 /* case-junk */ @ -> use();
      case 3 -> /* switch-empty-rule */ ;
      case 4:
      /* switch-label-sibling */ case : use();
      case String text when (/* guard-open */ ??? /* guard-close */) -> use();
    }
  }

  void missingIf(boolean value) { if (value) }
  void missingElse(boolean value) { if (value) {} else }
  void missingWhile(boolean value) { while (value) }
  void missingDo(boolean value) { do while (value); }
  void missingFor() { for (;;) }
  void missingEnhancedFor(int[] values) { for (int value : values) }
  void missingSwitch(int value) { switch (value) }
  void missingSynchronized(Object lock) { synchronized (lock) }

  void missingTry() {
    try
    catch (Exception exception)
    finally
  }

  void missingResourceTry(AutoCloseable resource) {
    try (resource)
    catch (Exception exception) {}
  }
}
