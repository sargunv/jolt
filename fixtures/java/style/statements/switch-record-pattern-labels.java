class SwitchRecordPatternLabels {
  Object f(Object o) {
    return switch (o) {
      case Point(int xCoordinateValue, int yCoordinateValue) when xCoordinateValue > 0 -> describePoint(xCoordinateValue, yCoordinateValue, "first quadrant", moreArguments);
      case Point(int x, int y) when x > 0 && someVeryLongConditionName(x, y) && another(y) -> 1;
      case Line(Point(int startXCoordinate, int startYCoordinate), Point end) -> length(end);
      case FirstVeryLongTypeName, SecondVeryLongTypeName, ThirdVeryLongTypeName, Fourth -> 1;
      default -> null;
    };
  }

  void g(Object o) {
    switch (o) {
      case Point(int xCoordinateValue, int yCoordinateValue) when xCoordinateValue > 0:
        run();
        break;
      default:
    }
  }
}
