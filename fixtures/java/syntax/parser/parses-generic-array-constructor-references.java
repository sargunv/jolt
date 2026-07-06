class GenericArrayConstructorReference {
    void method(java.util.Set<Class<?>> classes) {
        Class<?>[] copied = classes.toArray(Class<?>[]::new);
        java.util.function.IntFunction<java.util.List<String>[]> factory =
            java.util.List<String>[]::new;
    }
}
