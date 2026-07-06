class InvalidDeclarationContexts<T extends int> {
    void method(Object x, java.util.List<String> values) throws int {
        for (String value = null : values) {}
        for (String first, second : values) {}
        try (AutoCloseable missing, second = open()) {}
        try (AutoCloseable first = open(), second = open()) {}
        try {
            risky();
        } catch (int ex) {
        }
    }

    AutoCloseable open() { return null; }
    void risky() throws Exception {}
    transient void transientMethod() {}
    volatile InvalidDeclarationContexts() {}
    synchronized int synchronizedField;
}
