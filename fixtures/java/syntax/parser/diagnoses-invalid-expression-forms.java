class InvalidExpressions {
    void method(Object x, C c) {
        (f)();
        this();
        new C()();
        1 = x;
        a + b = c;
        (a) = b;
        new C;
        new C {};
        new int();
        Object invalidQualifiedCreation = new Outer<String>.Inner();
        Object validQualifiedCreation = new Outer.Inner<String>();
        int[] xs = new int[][3];
        int[] ys = new int[3] {1, 2};
        boolean primitiveInstanceof = x instanceof int;
    }
}
