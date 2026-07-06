class CatchParameterShape {
    void method() {
        try {
            risky();
        } catch (final java.io.IOException | RuntimeException ex) {
        }
    }

    void risky() throws java.io.IOException {}
}
