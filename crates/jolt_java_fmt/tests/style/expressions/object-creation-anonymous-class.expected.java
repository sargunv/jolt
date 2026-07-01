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
    return new /* ctor */ <String> Box("x");
  }

  Object annotated() {
    return new @Nonnull Box();
  }

  Object configured(User user, Account account, Settings settings) {
    return new Box(
      user.profile().displayName(),
      account.permissions().primaryRole(),
      settings.region().identifier()
    );
  }
}
