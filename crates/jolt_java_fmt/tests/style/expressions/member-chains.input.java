class Example {
void run(Builder builder, Object first, Object second) {
builder.add(first).add(second).build();
builder./* call */add(first);
builder.withFirstValue(first.reallyLongDisplayName()).withSecondValue(second.reallyLongDisplayName()).build();
this.field=builder.value;
this.field=builder./* field */value;
}
}
