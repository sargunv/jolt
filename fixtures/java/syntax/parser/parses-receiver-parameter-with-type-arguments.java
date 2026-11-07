class Outer<T, U> {
  void plain(Outer<T, U> this) {}

  void nested(Outer<T, java.util.Map<U, T>> this, int value) {}

  void other(java.util.Map<T, U> map, int value) {}
}
