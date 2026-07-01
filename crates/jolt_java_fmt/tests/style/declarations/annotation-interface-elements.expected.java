@interface Contract {
  String value() default "x"; // value marker

  int[] flags() default {1, 2};

  Nested nested() default @Nested(name = "demo", enabled = true);

  public abstract Class<?> type()[];

  class Helper {
    String name() {
      return "helper";
    }
  }

  enum Mode {
    AUTO,
    MANUAL,
  }

  @interface Meta {
    String label();
  }
}

@interface Nested {
  String name();

  boolean enabled() default false; // enabled marker
}
