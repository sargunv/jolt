enum ModifiedEnum {
    @Marker VALUE
}

class ModifiedConstructor {
    public ModifiedConstructor() {}
}

interface ModifiedInterface {
    public static final int X = 1;
}

@interface ModifiedAnnotation {
    @Marker String value();
}

@interface Marker {}
