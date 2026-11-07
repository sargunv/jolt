class VerbatimEdgeComments {
  void m() {
    call(a  b   /* after a malformed argument */);
    call(a  b   // ends the malformed argument's line
    );
  }

  static Thrown extends RuntimeException {  /* nothing */ }
}
