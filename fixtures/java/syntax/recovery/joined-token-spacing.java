class JoinedTokenSpacing extends Base<
    <Recovered, Argument>> {}

enum Unclosed {
  FIRST SECOND, THIRD

class AfterUnclosed {
  void missingTry() {
    try /* before a missing block */
    catch (Exception exception) {}
  }

  boolean guard(Object o) {
    return switch (o) {
      case String text when (/* open */ ??? /* close */) -> true;
      default -> false;
    };
  }
}
