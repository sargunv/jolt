record Point(@Deprecated int x, String... labels) implements Named {
    Point {
        labels = labels.clone();
    }

    public String name() {
        return labels[0];
    }
}

interface Named {}
