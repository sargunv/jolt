class FieldAccessResource {
    AutoCloseable existing;

    void method() throws Exception {
        try (this.existing) {}
    }
}
