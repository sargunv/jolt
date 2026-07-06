class Switches {
    void statement(Object value, int count) {
        switch (value) {
            case null, default -> {}
            case String s when s.isEmpty() -> s.trim();
            case Integer i -> {}
        }

        switch (count) {
            case 1:
            case 2:
                break;
            default:
                break;
        }
    }
}
