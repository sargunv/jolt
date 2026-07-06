class PrimitiveArrayMethodReference {
    void method() {
        java.util.function.IntFunction<int[]> factory = int[]::new;
    }
}
