fun throwsNothing() {
  throw // a comment ends the line and no operand follows it
  ;
}

fun throwsRecoveredOperand() {
  throw // a comment ends the line and the operand recovers a token
  )
}
