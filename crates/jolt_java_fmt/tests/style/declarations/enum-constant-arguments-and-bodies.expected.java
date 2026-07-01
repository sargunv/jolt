enum Mode {
  BASIC,
  @Marker
  SPECIAL(helper(1), new Box()) {
    void run() {
    }
  },
}
