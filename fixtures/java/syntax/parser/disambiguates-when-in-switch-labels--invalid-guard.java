class SwitchInvalidGuard {
    void method(int value) {
        switch (value) {
            case 1 when true:
                break;
        }
    }
}
