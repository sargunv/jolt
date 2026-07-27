@A // annotation
package p;

class TrailingLineBoundaries {
  String chain(java.util.List<String> values) {
    return values
        .stream() // step
        .findFirst()
        .orElse("");
  }
}
