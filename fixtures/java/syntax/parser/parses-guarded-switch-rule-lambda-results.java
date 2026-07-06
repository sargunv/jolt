class GuardedSwitchRuleLambdaResults {
    java.util.function.Function<Integer, Integer> method(Object value) {
        return switch (value) {
            case Boolean enabled when enabled -> x -> x + 1;
            default -> x -> x;
        };
    }
}
