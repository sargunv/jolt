class Types< /* params */
  @Marker T extends Number
  & // numeric
  Comparable<T>, // first
  U
> {
  java.util.@Marker List<@Marker String> @Marker [] names;

  void use(
    java.util.Map< /* args */
      String, // key
      ? super T
    > value
  ) {
  }

  void read(java.util.List<? extends Number> numbers) {
  }
}
