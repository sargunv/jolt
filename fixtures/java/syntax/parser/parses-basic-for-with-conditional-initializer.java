class BasicForConditionalInitializer {
    void method(boolean flag, int n) {
        int i;
        for (i = flag ? 1 : 2; i < n; i++) {}
    }
}
