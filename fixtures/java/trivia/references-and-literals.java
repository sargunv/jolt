class ReferencesAndLiterals {
  void run() {
    Class<?> type =
        java /* JOLT-TRIVIA:class-literal-fqn-1 */ . lang /* JOLT-TRIVIA:class-literal-fqn-2 */ . String
        /* JOLT-TRIVIA:class-literal-before-dot */ . class /* JOLT-TRIVIA:class-literal-before-semi */;
    java.util.function.Supplier<ReferencesAndLiterals> constructor =
        ReferencesAndLiterals /* JOLT-TRIVIA:ctor-ref-before-colons */ :: /* JOLT-TRIVIA:ctor-ref-after-colons */ new;
    java.util.function.Function<String, Integer> method =
        String /* JOLT-TRIVIA:method-ref-type */ :: /* JOLT-TRIVIA:method-ref-colons */ length;
    this /* JOLT-TRIVIA:explicit-type-target */ . <String /* JOLT-TRIVIA:explicit-type-arg */>
        generic /* JOLT-TRIVIA:explicit-type-name-before-paren */ ("x" /* JOLT-TRIVIA:explicit-type-call-arg */);
    java.util.List<String> list =
        new java /* JOLT-TRIVIA:new-fqn-1 */ . util /* JOLT-TRIVIA:new-fqn-2 */ . ArrayList
            < /* JOLT-TRIVIA:diamond-open */ > /* JOLT-TRIVIA:diamond-close-before-paren */ ();
  }

  <T> T generic(T value) {
    return value;
  }
}
