class InvalidLambdaParameters {
    void method() {
        java.util.function.BiFunction<Integer, Integer, Integer> mixedImplicit =
            (x, int y) -> y;
        java.util.function.BiFunction<Integer, Integer, Integer> mixedVar =
            (var x, y) -> y;
        java.util.function.BiFunction<String[], String, Integer> trailingVarargs =
            (String... values, String suffix) -> values.length;
        java.util.function.Function<Integer, Integer> finalImplicit =
            (final x) -> x;
        java.util.function.Function<Integer, Integer> annotatedImplicit =
            (@Deprecated x) -> x;
    }
}
