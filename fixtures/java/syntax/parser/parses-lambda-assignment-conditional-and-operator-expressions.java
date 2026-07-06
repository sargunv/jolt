class Operators {
    void method() {
        java.util.function.Function<String, String> trim = (String s) -> s.trim();
        java.util.function.IntUnaryOperator inc = x -> x + 1;
        int a = 1, b = 2, c = 3;
        a = b > c ? b + c * 2 : b | c ^ a & b;
    }
}
