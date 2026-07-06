record Box<T>(T value) {}

class Ambiguities<T extends java.util.Map<String, java.util.List<Integer>>> {
    Object field;

    void method(Object value) {
        var local = (String) value;
        java.util.function.Function<String, String> lambda = (String s) -> s;
        this.yield();
        Object access = this.field;
        java.util.Map<String, java.util.List<Integer>> nested = null;
        nested.get("key");
    }

    void yield() {}
}
