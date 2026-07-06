class ConditionalLambda {
    void method(boolean flag) {
        java.util.function.Function<Integer, Integer> existing = null;
        java.util.function.Function<Integer, Integer> chosen =
            flag ? existing : x -> x;
    }
}
