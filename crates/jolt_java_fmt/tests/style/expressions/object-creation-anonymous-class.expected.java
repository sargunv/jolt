class Example {
  Runnable make() {
    return new Runnable() {
      public void run() {
      }

      private int value() {
        return 1;
      }
    };
  }

  Object box() {
    return new <String> Box("x");
  }
}
