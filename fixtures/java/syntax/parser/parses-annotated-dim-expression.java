class AnnotatedDimExpression {
    void method(int n) {
        int[] values = new int @Marker [n];
    }
}

@interface Marker {}
