class ModifierLeadingCommentOwnership {
  // Malformed method modifier documentation.
  transient void method() {}

  // Malformed constructor modifier documentation.
  volatile ModifierLeadingCommentOwnership() {}

  // Malformed field modifier documentation.
  synchronized int field;

  void parameter(
      // Malformed parameter modifier documentation.
      transient String value) {}
}

// Malformed type modifier documentation.
transient class RecoveredType {}
