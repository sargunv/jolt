class Groups {
  int first;

  int second;
  int third;

  Groups() {
  }

  Groups(int value) {
  }

  static {
    initialize();
  }

  {
    warm();
  }

  void a() {
  }

  void b() {
  }

  class Nested {
  }

  interface NestedApi {
  }
}

record Range(int start, int end) {
  public Range {
    if (end < start) {
      throw new IllegalArgumentException();
    }
  }
}

interface Api {
  int VERSION = 1;

  String name();

  String label();

  class Helper {
  }
}

@interface Contract {
  String value();

  boolean enabled();

  class Helper {
  }

  @interface Nested {
  }
}
