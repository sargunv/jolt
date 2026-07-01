class Args {
  void call(String... names) {}
  void mix(final int count, Object... values) {}
  void annotated(@Nonnull String name, String @Marker ... labels) {}
}

record Values(String name, int... flags) {}
record AnnotatedValues(@Nonnull String name, String @Marker ... labels) {}
