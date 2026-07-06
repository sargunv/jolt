class Resources {
    void method() throws Exception {
        try (var declared = open(); existing) {}
    }

    AutoCloseable open() { return null; }
    AutoCloseable existing;
}
