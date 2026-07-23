@interface Values {
  String[] value();
}

@Values({
  "first", // Present separator.
  "last" // A synthesized comma here would become comment text.
})
class AnnotationValues {}

class ArrayValues {
  String[] values = {
    "first", // Present separator.
    "last" // A synthesized comma here would become comment text.
  };
}

enum EnumValues {
  FIRST, // Present separator.
  LAST // A synthesized comma here would become comment text.
}
