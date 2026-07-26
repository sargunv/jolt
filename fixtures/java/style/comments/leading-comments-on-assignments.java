class LeadingCommentsOnAssignments {
  int field;
  int[] values = new int[4];

  void shortAssignments(int x, int y, LeadingCommentsOnAssignments other) {
    // a line comment leads a short assignment
    x = y + 1;
    /* a block comment leads a short assignment */
    x += y + 1;
    // a line comment leads an array target
    values[0] = y + 1;
    // a line comment leads a field target
    other.field = y + 1;
    // a line comment leads a chained field target
    other.other().field = y + 1;
  }

  void longAssignments(int argumentOne, int argumentTwo, int anotherValue) {
    // the right-hand side still breaks when the assignment does not fit
    someLongTargetName = someLongMethodCall(argumentOne, argumentTwo) + anotherValue;
    // an already broken right-hand side stays broken
    someLongTargetName =
      someLongMethodCall(argumentOne, argumentTwo) + anotherValue;
  }

  LeadingCommentsOnAssignments other() {
    return this;
  }

  int someLongTargetName;

  int someLongMethodCall(int first, int second) {
    return first + second;
  }
}
