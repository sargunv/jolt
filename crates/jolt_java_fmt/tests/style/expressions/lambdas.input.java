class Example {
java.util.function.Supplier<String> supplier() {
return () -> "ready";
}
java.lang.Runnable runnable() {
return () -> { run(); };
}
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
java.util.function.Function<User, String> commentedMapper() {
return user /* selected */ -> user.name();
}
java.util.function.Function<User, String> commentedArrowMapper() {
return user -> /* body */ user.name();
}
java.util.function.Function<User, String> commentedTypedMapper() {
return (User user /* selected */) -> user.name();
}
java.util.function.Function<String[], Integer> commentedLengths() {
return (String... values /* all */) -> values.length;
}
void configure(Project project) {
project.getPluginManager().withPlugin("maven-publish", plugin -> { project.getExtensions().getByType(PublishingExtension.class).getPublications().withType(MavenPublication.class).configureEach(publication -> publication.versionMapping(mapping -> { mapping.allVariants(VariantVersionMappingStrategy::fromResolutionResult); })); });
}
}
