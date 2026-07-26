class ExtendsNoTypes extends //x
{}

class ImplementsNoTypes implements //x
{}

sealed class PermitsNoNames permits //x
{}

class SwitchDefaultLabel {
  int m(String s) {
    return switch (s) {
      case null, default //x
      -> 1;
    };
  }
}

class TryResourceSeparator {
  void m() throws Exception {
    try (; //x
    { }
  }
}
