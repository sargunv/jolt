class ParenthesizedPostfixIncrement {
  void statement(int value) {
    (value)++;
  }

  int expression(int value) {
    return (value)++ + 1;
  }

  int decrement(int value) {
    return (value)-- - 1;
  }
}
