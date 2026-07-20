class RecoveredExpressions {
  int negative = - -1;
  int positive = + +1;

  void lambdasAndArrays() {
    java.util.function.Function<Integer, Integer> conciseRecovered =
        item /* lambda-concise-sibling */ @ -> item;
    java.util.function.Function<Integer, Integer> parenthesizedRecovered =
        (item, /* lambda-parameter-sibling */ ,) -> item;
    java.util.function.Function<Integer, Integer> trailingLineComment =
        (first, second // lambda-last-parameter
        ) -> first;
    java.util.function.Function<Integer, Integer> trailingBlockComment =
        (first, second/* lambda-last-block */
        ) -> first;
    java.util.function.Function<Integer, Integer> closeLeadingComment =
        (first, second
        // lambda-close-leading
        ) -> first;
    int @Dimension [] array = new int[1] @Trailing [];
  }

  void methodReferences() {
    Object parenthesized = (/* receiver */)::target;
    Object primitive = void::target;
    Object postfix = value++::target;
    Object nested = value::target::nested;
  }

  void classLiterals() {
    Class<?> arrayType = String[].class;
    Class<?> objectCreation = new Object().class;
    Class<?> parenthesized = (value).class;
    Class<?> parenthesizedField = (value).field.class;
    Class<?> objectField = new Object().field.class;
    Class<?> literal = 1 .class;
    Class<?> voidArray = void[].class;
  }

  void memberComments() {
    Object
        // first suffix
        .something()
        .more();
  }

  boolean patterns(Object value) {
    boolean typed = value instanceof final @Readonly String item @Dimension [];
    boolean record = value instanceof Point(String name, Point(Integer x, _));
    boolean missingName = value instanceof Point(String);
    boolean invalidInitializer = value instanceof Point(String name = other);
    boolean invalidVar = value instanceof var item;
    boolean extraDeclarator = value instanceof Point(String first, second);
    boolean invalidRecordType = value instanceof int(String name);
    return typed || record || missingName || invalidInitializer || invalidVar || extraDeclarator || invalidRecordType;
  }
}
