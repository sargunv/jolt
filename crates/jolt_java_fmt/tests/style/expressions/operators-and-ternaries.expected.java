class Example {
  boolean allowed(User user, Account account) {
    return ( /* allowed */ user.isActive() && account.hasPermission("write"))
      ? true
      : false;
  }

  boolean commented(boolean ready) {
    return ready ? /* yes */ true : /* no */ false;
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
    allowed = left && /* both */ right;
    count += 1;
    count += /* increment */ 1;
    allowed =
      user.isActive()
      && account.hasPermission("write")
      && featureFlags.enabled()
      && auditPolicy.allows(user);
  }
}
