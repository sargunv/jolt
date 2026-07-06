class ArrayAccessForms {
    void method(int[] values) {
        int byName = values[0];
        int byPrimary = (values)[0];
        int byCreation = new int[] {1}[0];
    }
}
