class ReadabilityParentheses {
  int leading = a | /* before the operand */ b & c;
  int shifted = a & /* before the shift */ b << 1;
  int nested = a << /* before the sum */ b + c;
  int inner = a << b /* inside the sum */ + c;
  int after = a | b & c /* after the operand */ | d;

  void lineComment() {
    x = a & b // ends the left operand
        | c;
  }
}
