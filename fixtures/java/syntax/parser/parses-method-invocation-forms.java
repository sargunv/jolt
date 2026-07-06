class MethodInvocations extends Base {
    static void staticMethod() {}
    static <T> void staticGeneric(T value) {}
    void simple() {}
    void instance() {}
    <T> void generic(T value) {}

    void method(MethodInvocations target) {
        simple();
        MethodInvocations.staticMethod();
        target.instance();
        (target).instance();
        super.baseMethod();
        MethodInvocations.super.baseMethod();
        this.<String>generic("value");
        target.<String>generic("value");
        MethodInvocations.<String>staticGeneric("value");
        super.<String>baseGeneric("value");
        MethodInvocations.super.<String>baseGeneric("value");
    }
}

class Base {
    void baseMethod() {}
    <T> void baseGeneric(T value) {}
}
