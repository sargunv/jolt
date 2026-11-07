class MissingOperandBeforeClose {
  void returns() {
    return
    // the operand is missing and the block closes
  }

  void throwsBlock() {
    throw
    /* the operand is missing */
  }

  void throwsDoc() {
    throw
    /** the operand is missing */
  }

  void returnsTokens() {
    return a b { c } d
    // recovery stops at the closing brace
  }
}
