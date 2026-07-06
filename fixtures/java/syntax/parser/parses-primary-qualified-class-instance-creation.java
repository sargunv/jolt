class PrimaryQualifiedCreation {
    void method() {
        Object inner = new Outer().new Inner();
    }
}

class Outer {
    class Inner {}
}
