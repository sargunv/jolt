enum OnlySeparator { /* a */ // b
  ;
}

enum EmptyWithComments { /* a */ /* b */ ; }

class CommentRunsAroundRemovedTokens {
  void method() {
    label: {} /* a */ // b
    ;
    for ( /* a */ /* b */ ;;) {
      break;
    }
    run(); /* a */ // b
    try /* a */ // b
    (var in = open()) {}
  }
}
