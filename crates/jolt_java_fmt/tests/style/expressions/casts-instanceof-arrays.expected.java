class Example {
  boolean matches(Object value) {
    return (String) value instanceof String text
      || value instanceof java.util.List<?>;
  }

  int[] values(int size, int count) {
    return new int[size][count];
  }

  String[] names() {
    return new String[] {"a", "b"};
  }

  Object literals() {
    return String[][].class == int.class ? void.class : this.getClass();
  }

  Object qualified(Inner inner) {
    return Example.this == inner.superValue ? super.toString() : this.toString();
  }
}
