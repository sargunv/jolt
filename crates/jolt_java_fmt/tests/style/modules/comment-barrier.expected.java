module demo {
  requires z.lib;

  // keep requires barrier
  requires a.lib;

  exports z.api;

  // keep exports barrier
  exports a.api;
}
