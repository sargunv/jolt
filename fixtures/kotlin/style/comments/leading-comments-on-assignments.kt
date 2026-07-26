class LeadingCommentsOnAssignments {
  var field = 0
  val values = IntArray(4)
  var someLongTargetName = 0

  fun shortAssignments(y: Int, other: LeadingCommentsOnAssignments) {
    var x = 0
    // a line comment leads a short assignment
    x = y * 2
    /* a block comment leads a short assignment */
    x += y * 2
    // a line comment leads an index target
    values[0] = y * 2
    // a line comment leads a navigation target
    other.field = y * 2
    // a line comment leads a chained navigation target
    other.other().field = y * 2
  }

  fun longAssignments(argumentOne: Int, argumentTwo: Int, anotherValue: Int) {
    // the right-hand side still breaks when the assignment does not fit
    someLongTargetName = someLongFunctionCall(argumentOne, argumentTwo) + anotherValue
    // an already broken right-hand side stays broken
    someLongTargetName =
      someLongFunctionCall(argumentOne, argumentTwo) + anotherValue
  }

  fun other(): LeadingCommentsOnAssignments = this

  fun someLongFunctionCall(first: Int, second: Int): Int = first + second
}
