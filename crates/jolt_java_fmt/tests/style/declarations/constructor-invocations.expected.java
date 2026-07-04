class Example extends Base {
  Example() {
    super(1, 2); // base args
    int x = 3;
  }

  Example(String value) {
    <String>this(value, 0); // delegate
  }

  Example(String configLocation) {
    this(new String[] { configLocation }, true, null);
  }

  Example(Object text) {
    int value = Integer.parseInt(text.toString());
    super(value);
  }

  Example(User user, Account account, Settings settings) {
    this(
      user.profile().displayName(),
      account.permissions().primaryRole(),
      settings.region().identifier()
    );
  }

  class Inner extends Base {
    Inner(Example outer) {
      outer.<String>super("x"); // qualified base
    }
  }
}
