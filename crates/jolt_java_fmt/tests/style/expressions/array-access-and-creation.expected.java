class Example {
  Object pick(Object[] values, int index) {
    return values[index];
  }
  Object create(Object first, Object second) {
    return new Outer<String>().new Inner(first, second);
  }
}
