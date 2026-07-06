class AssignmentLeftHandSides {
    int field;

    void method(int[] values) {
        int local;
        local = 1;
        this.field = 2;
        values[0] = 3;
    }
}
