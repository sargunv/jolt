class SwitchGroupEmptyStatement {
  void groups(int value) {
    switch (value) {
      case 1: ;
      case 2:
        run();
        ;
      case 3:
      default: ;
    }
  }
}
