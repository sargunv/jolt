@interface Contract {
    String value() default "x";
    int[] flags() default {1, 2};
    Nested nested() default @Nested;
}

@interface Nested {}
