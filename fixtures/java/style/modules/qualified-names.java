open module com . example . app {
  provides com . example . Service with com . example . impl . ServiceImpl;
  exports com . example . api to z . Module, a . Module;
  requires java . base;
}
