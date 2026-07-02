class Example {
  int before = 1;
  // @formatter:off
  int rawField=1+2;
  // @formatter:on
  int afterField = 2;

  void run() {
    if (true) {
      // @formatter:off
      int rawLocal=3+4;
      call( a,b );
      // @formatter:on
      int afterLocal = 5;
    }
  }
}
