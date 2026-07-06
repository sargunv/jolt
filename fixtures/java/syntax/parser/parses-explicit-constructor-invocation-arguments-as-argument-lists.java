class ConstructorInvocationArguments extends Base {
    ConstructorInvocationArguments() {
        this(0);
    }

    ConstructorInvocationArguments(String value) {
        <String>this(value, 0);
    }

    <T> ConstructorInvocationArguments(T value, int marker) {
        super(value);
    }

    class Inner extends Base {
        Inner(ConstructorInvocationArguments outer) {
            outer.super(0);
        }

        Inner(ConstructorInvocationArguments outer, String value) {
            outer.<String>super(value);
        }

        Inner() {
            (new ConstructorInvocationArguments()).super(0);
        }

        Inner(String value) {
            (new ConstructorInvocationArguments()).<String>super(value);
        }
    }
}

class Base {
    Base() {}
    Base(int value) {}
    <T> Base(T value) {}
}
