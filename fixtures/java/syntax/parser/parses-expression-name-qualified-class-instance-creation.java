class QualifiedCreation {
    Outer outer;

    void method() {
        Object inner = outer.new Inner();
    }
}

class Outer {
    class Inner {}
}
