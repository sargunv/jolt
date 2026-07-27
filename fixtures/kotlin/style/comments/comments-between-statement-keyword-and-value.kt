class CommentsBetweenStatementKeywordAndValue {
  fun throws1() {
    throw // a comment ends the line after the keyword
    IllegalStateException()
  }

  fun throwsInline() {
    throw /* a block comment stays inline */ IllegalStateException()
  }

  fun throwsOnTheNextLine() {
    throw
    IllegalStateException()
  }

  fun throwsLong() {
    throw // a long value still wraps below the keyword
    IllegalStateException(someLongFunctionCall(firstArgumentName, secondArgumentName))
  }

  fun throwsAfterSameLineCommentRun(value: Throwable): Nothing {
    throw /* block */ // line
    value
  }

  // A newline ends a `return`, a `break`, and a `continue`, so nothing below the
  // keyword's comment is its value: it is a statement of its own and stays at
  // statement indent. Only `throw` continues across the newline.
  fun returns(y: Int) {
    return // a comment ends the line after the keyword
    y + 1
  }

  fun returnsAfterBlockComment(y: Int) {
    return /* the newline, not the comment, is what ends the return */
    y + 1
  }

  fun returnsInline(y: Int): Int {
    return /* a block comment stays inline */ y + 1
  }

  fun returnsToLabel(y: Int) {
    run {
      return@run /* a block comment stays inline */ y + 1
    }
  }

  fun breaksToLabel(y: Int) {
    outer@ while (ready()) {
      if (y > 1) break@outer // a comment after the label stays there
      report(y)
    }
  }

  fun continuesToLabel(y: Int) {
    outer@ while (ready()) {
      if (y > 1) continue@outer /* a block comment stays there */
      report(y)
    }
  }

  fun ready(): Boolean = true

  fun report(value: Int) {}

  fun someLongFunctionCall(first: Int, second: Int): String = "$first and $second"

  val firstArgumentName = 3
  val secondArgumentName = 4
}
