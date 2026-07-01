module demo {
  requires beta;

  uses z.Service;

  // @formatter:off
  provides z.Service with b.Impl,a.Impl;
  opens   raw.pkg  to   x.y,z.y;
  // @formatter:on
  requires alpha;

  exports api;
}
