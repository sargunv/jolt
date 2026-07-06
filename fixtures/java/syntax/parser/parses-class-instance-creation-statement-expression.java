class CreationStatementExpression {
    void method() {
        new Object();
        new Runnable() {
            public void run() {}
        };
    }
}
