class InvalidStatementExpressions {
    void method(int i, int j) {
        1 + 2;
        i;
        for (i + 1; i < 10; j + 1) {}
    }
}
