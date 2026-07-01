class Groups {
  int first;


  int second;
  int third;
  void a() {}
  void b() {}
}
record Range(int start, int end) {
  public Range { if (end < start) throw new IllegalArgumentException(); }
}
