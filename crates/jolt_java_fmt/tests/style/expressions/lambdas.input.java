class Example {
java.util.function.Function<User, String> mapper() {
return (user) -> user.name();
}
java.util.function.Function<User, String> typedMapper() {
return (User user) -> user.name();
}
java.util.function.Function<User, User> varMapper() {
return (var user) -> user;
}
java.util.function.BiFunction<Integer, Integer, Integer> adder() {
return (left,right) -> left + right;
}
java.util.function.Function<String[], Integer> lengths() {
return (String... values) -> values.length;
}
java.util.function.Function<User, User> annotatedMapper() {
return (@Nonnull User user) -> user;
}
java.util.function.Function<User, User> finalMapper() {
return (final User user) -> user;
}
java.util.function.Function<String[], Integer> annotatedLengths() {
return (String @Marker ... values) -> values.length;
}
java.util.function.IntUnaryOperator unnamed() {
return (_) -> 0;
}
}
