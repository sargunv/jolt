// a detached comment on a removed top-level semicolon

;
class RemovedSemicolonBlankLines {
  int x;
  // a detached comment on a removed member semicolon

  ;
  int y;

  void method() {
    a();
    // a detached comment on a removed empty statement

    ;
    b();
  }
}
