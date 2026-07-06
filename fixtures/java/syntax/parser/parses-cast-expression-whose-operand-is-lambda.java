class CastLambda {
    void method() {
        Runnable runnable = (Runnable) () -> {};
    }
}
