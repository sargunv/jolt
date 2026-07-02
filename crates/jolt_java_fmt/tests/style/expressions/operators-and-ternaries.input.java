class Example {
boolean allowed(User user, Account account) {
return (/* allowed */ user.isActive()&&account.hasPermission("write"))?true:false;
}
boolean commented(boolean ready) {
return ready?/* yes */true:/* no */false;
}
boolean returned() {
return user.isActive() && account.hasPermission("write") && featureFlags.enabled() && auditPolicy.allows(user);
}
void guarded() {
if (user.isActive() && account.hasPermission("write") && featureFlags.enabled() && auditPolicy.allows(user)) run();
}
void assign(boolean left, boolean right, User user, Account account, FeatureFlags featureFlags, AuditPolicy auditPolicy) {
allowed=left&&right;
allowed=left&&/* both */right;
count+=1;
count+=/* increment */1;
allowed = user.isActive() && account.hasPermission("write") && featureFlags.enabled() && auditPolicy.allows(user);
}
float estimate(float currentEstimate) {
currentEstimate = (currentEstimate + xxxxxxxxxxxxx / currentEstimate) / 2.0f;
return currentEstimate;
}
int sum(int x) {
return x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x + x;
}
int nestedTernary(int i) {
int j = i == 0 ? 0 : i == 1 ? 1 : i == 2 ? 2 : i == 3 ? 3 : i == 4 ? 4 : i == 5 ? 5 : i == 6 ? 6 : i == 7 ? 7 : i;
return j;
}
}
