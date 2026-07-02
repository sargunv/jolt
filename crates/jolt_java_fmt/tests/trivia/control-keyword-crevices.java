class ControlKeywordCrevices {
  void run(boolean flag) {
    outer /* JOLT-TRIVIA:label-before-colon */ : /* JOLT-TRIVIA:label-after-colon */
    while /* JOLT-TRIVIA:while-before-paren */ (flag /* JOLT-TRIVIA:while-before-close */)
      break /* JOLT-TRIVIA:break-label-gap */ outer /* JOLT-TRIVIA:break-before-semi */;

    do /* JOLT-TRIVIA:do-before-body */ {
      flag = false;
    } while /* JOLT-TRIVIA:do-while-before-paren */ (flag /* JOLT-TRIVIA:do-while-before-close */)
        /* JOLT-TRIVIA:do-while-before-semi */;

    synchronized /* JOLT-TRIVIA:synchronized-before-paren */ (
        this /* JOLT-TRIVIA:synchronized-before-close */) /* JOLT-TRIVIA:synchronized-before-body */ {
      assert /* JOLT-TRIVIA:assert-before-condition */ flag
          : /* JOLT-TRIVIA:assert-before-message */ "message" /* JOLT-TRIVIA:assert-before-semi */;
    }
  }
}
