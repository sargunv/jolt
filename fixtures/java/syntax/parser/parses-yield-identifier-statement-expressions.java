class YieldIdentifierStatementExpressions {
    void method() {
        yield = 1;
        yield += 2;
        yield[0] = 3;
        yield++;
        yield.foo();
    }
}
