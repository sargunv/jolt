class SelectorRelationalExpressions {
    int index;
    java.util.List<String> items;

    boolean method(String[] names) {
        return this.index < names.length
            && names[0].length() < this.items.size()
            && this.items.size() < names.length;
    }
}
