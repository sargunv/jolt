@interface Contract { String value() default "x"; int[] flags() default {1,2}; Nested nested() default @Nested(name="demo", enabled=true); public abstract Class<?> type()[]; }

@interface Nested { String name(); boolean enabled() default false; }
