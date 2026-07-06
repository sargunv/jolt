class ConstructorTypeArgumentsCreation {
    void method() {
        Object box = new <String> Box("value");
    }
}

class Box<T> {
    <U> Box(U value) {}
}
