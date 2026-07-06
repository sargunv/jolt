class Types<T extends Number & Comparable<T>> {
    int primitive;
    double floating;
    java.util.Map<String, ? extends Number> upper;
    java.util.List<? super T>[] lower;
    T[][] matrix;
}
