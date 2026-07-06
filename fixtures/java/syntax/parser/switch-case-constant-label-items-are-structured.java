class SwitchCaseConstants {
    static final int NAME = 3;

    void method(int value) {
        switch (value) {
            case 1, -2, NAME -> value++;
        }
    }
}
