sealed class A<T extends B & C> extends B implements I permits D {
    @Deprecated
    private int first = 1, second[] = {2};

    static {}
    {}

    A() {
        this(0);
    }

    A(int value) {}

    <U> void method(final int x, String... rest) throws E {}
    void receiver(A this) {}
    class Inner {
        void receiver(A A.this) {}
    }
}

final class D extends A {}
class B {}
interface C {}
interface I {}
class E extends Exception {}
