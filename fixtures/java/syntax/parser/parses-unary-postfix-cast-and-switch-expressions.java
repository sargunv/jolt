class ExpressionEdges {
    int method(Object value) {
        int i = 0;
        ++i;
        --i;
        i++;
        i--;
        int cast = (int) value;
        return switch (value) {
            case Integer n -> {
                yield n;
            }
            case Character c -> {
                yield (char) c;
            }
            default -> -i;
        };
    }
}
