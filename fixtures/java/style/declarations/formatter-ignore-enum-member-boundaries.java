enum MemberOnly {
  A;
  // @formatter:off
  int raw=1+2;
  // @formatter:on
  int after=3;
}

enum CrossingPartitions {
  A,
  // @formatter:off
  B;
  int raw=1+2;
  // @formatter:on
  int after=3;
}
