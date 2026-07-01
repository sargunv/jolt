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
}
