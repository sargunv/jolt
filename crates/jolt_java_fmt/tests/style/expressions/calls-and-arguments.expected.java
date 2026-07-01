class Example {
  void run(Target target, User user, Account account) {
    print(user.name(), account.id());
    target.accept(user -> user.name(), account.hasPermission("write"));
    this.<String>convert(user.name());
  }
}
