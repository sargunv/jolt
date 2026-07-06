class Flow {
    int method(java.util.List<String> values) throws Exception {
        while (values.isEmpty()) continue;
        do { break; } while (false);
        for (int i = 0; i < 10; i++) {}
        for (String value : values) {}
        for (SomeClass<?> value : values) {}
        synchronized (this) {}
        try {
            throw new Exception();
        } catch (java.io.IOException | RuntimeException ex) {
            return 1;
        } finally {
            values.clear();
        }
        try (var ignored = open(); existing) {}
        return 0;
    }

    AutoCloseable open() { return null; }
    AutoCloseable existing;
}
