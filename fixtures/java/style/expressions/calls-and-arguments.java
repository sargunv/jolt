class Example {
Result build(User user, Account account) {
return create(user.profile().displayName(),account.permissions().primaryRole(),settings.region().identifier());
}
void run(Target target, User user, Account account) {
print(user.name(),account.id());
target.accept((user) -> user.name(),account.hasPermission("write"));
this.<String>convert(user.name());
target.accept(user.profile().displayName(),account.permissions().primaryRole(),settings.region().identifier());
target.accept(user.profile().displayName(),

account.permissions().primaryRole(),

settings.region().identifier());
log(/* nothing */);
log(user /* selected */);
log(user,/* account */account);
log(user, // selected
account);
log(user
// before close
);
log(
// no args
);
}
}
