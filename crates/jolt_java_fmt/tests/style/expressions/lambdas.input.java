class Example {
java.util.function.Function<User, String> mapper() {
return (user) -> user.name();
}
java.util.function.Function<User, String> typedMapper() {
return (User user) -> user.name();
}
}
