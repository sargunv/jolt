record Pair(int left, int right) {}

class Patterns {
    int method(Object value) {
        if (value instanceof Pair(int left, _)) {
            return left;
        }
        return switch (value) {
            case Pair(int left, int right) -> left + right;
            case String text when !text.isEmpty() -> text.length();
            default -> 0;
        };
    }
}
