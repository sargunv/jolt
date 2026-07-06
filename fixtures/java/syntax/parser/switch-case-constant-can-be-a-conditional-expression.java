class SwitchCaseConstantExpression {
    static final int OFFSET = 1;

    void method(int value) {
        switch (value) {
            case 1 + OFFSET -> value++;
        }
    }
}
