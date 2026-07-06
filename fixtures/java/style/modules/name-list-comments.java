module demo {
  provides com.example.Plugin with com.example.impl.PluginImpl, // first impl
  com.example.impl.PluginFallback;
  opens com.example.internal to friend.one, // first open
  friend.two;
  exports com.example.api to partner.one, // first export
  partner.two;
}
