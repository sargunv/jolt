class SwitchCasePatternGuard {
    void method(Object value) {
        switch (value) {
            case String s when s.isEmpty() -> s.trim();
            case String s when (s.isBlank()) -> s.trim();
        }
    }
}
