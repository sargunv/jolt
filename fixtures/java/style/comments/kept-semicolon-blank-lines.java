// a detached comment on a kept top-level semicolon

;
class KeptSemicolonBlankLines {
  int x;
  // a detached comment on a kept member semicolon

  ;
  int y;

  void method() {
    a();
    // a detached comment on a kept empty statement

    ;
    b();
  }
}
