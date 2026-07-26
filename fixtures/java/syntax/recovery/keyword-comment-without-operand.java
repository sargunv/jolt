class KeywordCommentWithoutOperand {
  void throwsNothing() {
    throw // a comment ends the line and no operand follows it
    ;
  }

  void throwsRecoveredOperand() {
    throw // a comment ends the line and the operand recovers a token
    );
  }

  void assertsNothing() {
    assert // a comment ends the line and no condition follows it
    ;
  }

  void assertsNothingWithMessage() {
    assert // a comment ends the line and only the message follows it
    : "message";
  }

  int yieldsRecoveredOperand(int value) {
    return switch (value) {
      default -> {
        yield // a comment ends the line and the operand recovers a token
        );
      }
    };
  }
}
