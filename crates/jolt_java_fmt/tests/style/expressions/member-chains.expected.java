class Example {
  void run(Builder builder, Object first, Object second) {
    builder.add(first).add(second).build();
    this.field = builder.value;
  }
}
