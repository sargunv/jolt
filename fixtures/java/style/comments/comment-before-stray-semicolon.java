class CommentBeforeStraySemicolon {
  void a() {} // about a
  ;
  void b() {}

  void statements(boolean x, boolean y, int z) {
    if (x) {} // about the if
    ;
    while (y) {
      run();
    } // about the while
    ;
    switch (z) {
      case 1: // about case 1
        ;
      case 2:
        {} // about the block
        ;
      default:
    }
  }
}

class A {} // about A
;
class B {}
