@interface Contract {
  String value() default "x"; // value marker

  int[] flags() default {
    /* start */
    1, // one
    2, /* two */ 3, // trailing
  };

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
