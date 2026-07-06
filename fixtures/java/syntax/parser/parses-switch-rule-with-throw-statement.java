class ThrowSwitchRule {
    int method(Object value) {
        return switch (value) {
            case null -> throw new IllegalArgumentException();
            default -> 0;
        };
    }
}
