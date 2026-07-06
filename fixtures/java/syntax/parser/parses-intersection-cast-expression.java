class IntersectionCast {
    void method(Object value) {
        Runnable runnable = (Runnable & AutoCloseable) value;
    }
}
