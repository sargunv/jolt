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
boolean hasMore(String[] names) {
return this.index<names.length&&names[0].length()<items.size()&&items.size()<names.length;
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
}
