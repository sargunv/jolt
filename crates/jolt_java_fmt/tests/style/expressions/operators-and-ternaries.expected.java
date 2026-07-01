class Example {
  boolean allowed(User user, Account account) {
    return (user.isActive() && account.hasPermission("write")) ? true : false;
  }
  void assign(boolean left, boolean right) {
    allowed = left && right;
    count += 1;
  }
}
