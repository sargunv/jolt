class NullDefaultSwitchLabel {
    int method(Object value) {
        return switch (value) {
            case null, default -> 0;
        };
    }
}
