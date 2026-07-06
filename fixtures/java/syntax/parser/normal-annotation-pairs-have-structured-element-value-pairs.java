@Normal(name = "test", nested = @Marker, values = {1, 2})
class Annotated {}

@interface Normal {
    String name();
    Marker nested();
    int[] values();
}

@interface Marker {}
