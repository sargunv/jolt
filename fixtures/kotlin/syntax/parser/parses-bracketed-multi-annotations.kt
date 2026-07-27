annotation class FirstAnnotation
annotation class SecondAnnotation

@[FirstAnnotation] fun annotated() {}

@field:[FirstAnnotation SecondAnnotation]
val value = 1
