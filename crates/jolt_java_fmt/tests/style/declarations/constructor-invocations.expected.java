class Example extends Base {
  Example() {
    super(1, 2); // base args
    int x = 3;
  }

  Example(String value) {
    <String>this(value, 0); // delegate
  }

  class Inner extends Base {
    Inner(Example outer) {
      outer.<String>super("x"); // qualified base
    }
  }
}
