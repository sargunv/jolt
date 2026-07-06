class CatchUnionTypeShape {
    void method() {
        try {
            risky();
        } catch (java.io.IOException | RuntimeException ex) {
        }
    }

    void risky() throws java.io.IOException {}
}
