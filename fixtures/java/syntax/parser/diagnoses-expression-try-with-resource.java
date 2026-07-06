class ExpressionTryWithResource {
    void method() throws Exception {
        try (open()) {}
    }

    AutoCloseable open() { return null; }
}
