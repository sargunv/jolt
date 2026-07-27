val short = { @Mark
  value: Int -> value }

val medium = {
  @Configured("moderate")
    value: MediumType ->
    value
}

val long = {
  @AnnotationWithAnExceptionallyLongName("an-exceptionally-long-argument")
    value: LongType ->
    value
}
