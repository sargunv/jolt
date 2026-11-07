public @Deprecated class Hoisted {
  static public final @Nonnull String NAME = "x";
  @Inject final protected @Nullable Object field;
  synchronized static public <T> T generic() { return null; }
  public @A static <T> T annotatedGeneric() { return null; }
  public @Inject Hoisted() {}
  abstract static protected class Nested {}
  native public static void nativeMethod();
  volatile transient private int flags;
  final static strictfp sealed abstract class Shape permits Circle {}
  static non-sealed @Marker final class Circle extends Shape {}
  default public void unused() {}
}

interface Defaults {
  default public void run() {}
  static public void helper() {}
}

final /* first */ public static class Barrier {
  static public final /* stays */ protected int value;
  public // own line
  static void split() {}
}

enum Mode { A; static public final int X = 1; }

record Point(int x, int y) { public @Inject Point {} }

@interface Config { public @Nonnull abstract String value(); }
