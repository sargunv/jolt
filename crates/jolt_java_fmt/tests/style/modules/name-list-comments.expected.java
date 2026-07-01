module demo {
  exports com.example.api to
    partner.one, // first export
    partner.two;

  opens com.example.internal to
    friend.one, // first open
    friend.two;

  provides com.example.Plugin with
    com.example.impl.PluginImpl, // first impl
    com.example.impl.PluginFallback;
}
