class SwitchGroupSemicolonComments {
  void run(int x) {
    switch (x) {
      case 1:
        run(); ; /*a*/
        ; // b
      case 2:
        run();
        ; // last
      default:
        ; /*a*/ ; // b
    }
  }
}
