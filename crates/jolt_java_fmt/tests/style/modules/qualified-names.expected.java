open module com.example.app {
  requires java.base;

  exports com.example.api to z.Module, a.Module;

  provides com.example.Service with com.example.impl.ServiceImpl;
}
