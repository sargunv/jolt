class TernarySwitchRuleLabels {
  String guarded(Object value, boolean condition) {
    return switch (value) {
      case Integer number when condition ? true : false -> "integer";
      default -> "other";
    };
  }

  int constant(int value) {
    return switch (value) {
      case true ? 1 : 2 -> 10;
      default -> 0;
    };
  }
}
