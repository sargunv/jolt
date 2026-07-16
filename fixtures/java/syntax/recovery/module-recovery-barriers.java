module recovery.barriers {
  exports z.api target;
  exports a.api to alpha.target;
  opens z.internal target;
  opens a.internal to alpha.target;
  provides z.Service implementation.Type;
  provides a.Service with alpha.Implementation;
}
