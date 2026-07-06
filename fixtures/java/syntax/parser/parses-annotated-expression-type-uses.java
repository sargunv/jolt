class AnnotatedExpressionTypeUses {
    void method(Object o) {
        new @A(0x44) ArrayList<>();
        java.util.function.Supplier<java.util.List<?>> a = @A(0x45) ArrayList::new;
        java.util.function.Supplier<java.util.List<?>> b = @A(0x46) ImmutableList::of;
        String s = (@A(0x47) String) o;
        java.util.List<?> xs = new ArrayList<@A(0x48) String>();
        xs = ImmutableList.<@A(0x49) String>of();
        java.util.function.Supplier<java.util.List<?>> c = ArrayList<@A(0x4A) String>::new;
        java.util.function.Supplier<java.util.List<?>> d = ImmutableList::<@A(0x4B) String>of;
    }
}

@interface A {
    int value();
}
