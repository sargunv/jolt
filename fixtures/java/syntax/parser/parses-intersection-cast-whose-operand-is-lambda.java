interface CloseRunnable extends Runnable, AutoCloseable {}

class IntersectionCastLambda {
    void method() {
        CloseRunnable runnable = (Runnable & AutoCloseable) () -> {};
    }
}
