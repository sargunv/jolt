class ConstructorInvocationArguments {
    ConstructorInvocationArguments(String value) {
        this(new String[] {value}, true, null);
    }

    ConstructorInvocationArguments(String[] values, boolean flag, Object extra) {}
}
