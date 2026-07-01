class Args {
  void call(String... names) {}
  void mix(final int count, Object... values) {}
}

record Values(String name, int... flags) {}
