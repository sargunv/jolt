module demo {
  provides com.example.Plugin with com.example.impl.PluginImpl;
  opens com.example.internal;
  requires transitive static com.example.lib;
  exports com.example.api;
  uses com.example.Plugin;
  requires java.sql;
}
