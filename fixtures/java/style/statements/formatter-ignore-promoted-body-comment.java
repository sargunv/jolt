class FormatterIgnorePromotedBodyComment {
  void m(boolean a) {
    // @formatter:off
    if (a)  return;    // trails the unbraced body, which stays unbraced
    // @formatter:on
    if (a) return; // trails the body inside its promoted braces
  }
}
