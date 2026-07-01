class Example {
  boolean matches(Object value) {
    return (String) value instanceof /* target */ String text
      || value instanceof java.util.List<?>;
  }

  Object casted(Object value) {
    return ( /* target */ String) /* cast */ value;
  }

  Object intersection(Object value) {
    return (Runnable & AutoCloseable) value;
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
    return Example. /* this */ this == inner.superValue
      ? Example. /* super */ super.toString()
      : this.toString();
  }
}
