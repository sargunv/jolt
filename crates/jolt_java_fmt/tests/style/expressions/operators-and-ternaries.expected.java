class Example {
  boolean allowed(User user, Account account) {
    return (user.isActive() && account.hasPermission("write")) ? true : false;
  }

  void assign(
    boolean left,
    boolean right,
    User user,
    Account account,
    FeatureFlags featureFlags,
    AuditPolicy auditPolicy
  ) {
    allowed = left && right;
    count += 1;
    allowed =
      user.isActive()
      && account.hasPermission("write")
      && featureFlags.enabled()
      && auditPolicy.allows(user);
  }
}
