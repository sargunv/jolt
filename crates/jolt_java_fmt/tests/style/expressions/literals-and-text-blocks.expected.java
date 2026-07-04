class Example {
  String escaped() {
    return "line\\n" + '\t';
  }

  String block() {
    return """
        alpha
          beta
        """;
  }

  String template(String name) {
    return STR."Hello \{name}";
  }

  int numeric() {
    return 1_000 + 0x0f;
  }
}
