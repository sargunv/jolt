module demo {
  requires static transitive com.example.lib;
  requires java.sql;

  exports com.example.api;

  opens com.example.internal;

  uses com.example.Plugin;

  provides com.example.Plugin with com.example.impl.PluginImpl;
}
