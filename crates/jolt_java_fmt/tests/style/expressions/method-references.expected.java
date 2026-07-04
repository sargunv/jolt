class Example extends Base {
  void run(String value, Example target, java.util.Set<Class<?>> classes) {
    var expressionName = value::length;
    var primary = (value)::trim;
    var referenceType = String::length;
    var superReference = super::toString;
    var qualifiedSuper = Example.super::toString;
    var constructor = Example::new;
    var arrayConstructor = String[]::new;
    var primitiveArray = int[]::new;
    var genericArrayConstructor = classes.toArray(Class<?>[]::new);
    var genericArrayFactory = java.util.List<String>[]::new;
    var generic = target::<String>id;
    var typeGeneric = Example::<String>staticId;
    var constructorGeneric = java.util.ArrayList<String>::<String>new;
    var commentedTarget = value:: /* target */ length;
    var commentedConstructor = Example::new /* constructor */;
    var commentedSeparator = Example:: /* constructor */ new;
  }

  <T> T id(T value) {
    return value;
  }

  static <T> T staticId(T value) {
    return value;
  }
}

class Base {
}
