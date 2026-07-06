@StringSingle("test")
@NestedSingle(@Marker)
@ArraySingle({@Marker, "x"})
class Annotated {}

@interface StringSingle {
    String value() default "fallback";
}

@interface NestedSingle {
    Marker value() default @Marker;
}

@interface ArraySingle {
    Object[] value() default {@Marker, "y"};
}

@interface Marker {}
