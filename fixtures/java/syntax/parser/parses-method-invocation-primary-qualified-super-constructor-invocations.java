class MethodInvocationQualifiedSuper {
    Inner makeOuter() {
        return null;
    }

    class Inner extends Base {
        Inner() {
            makeOuter().super(0);
        }

        Inner(String value) {
            makeOuter().<String>super(value);
        }
    }
}

class Base {
    Base() {}
    Base(int value) {}
    <T> Base(T value) {}
}
