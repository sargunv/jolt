class VarargsLambdaParameter {
    void method() {
        java.util.function.Function<String[], Integer> lengths =
            (String... values) -> values.length;
    }
}
