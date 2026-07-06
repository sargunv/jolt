class Base {
    Base(int value) {}
}

class FlexibleConstructor extends Base {
    FlexibleConstructor(String text) {
        int value = Integer.parseInt(text);
        super(value);
    }
}
