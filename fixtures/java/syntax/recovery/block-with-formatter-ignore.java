class C { void m() {
  int before;
  // @formatter:off
  int raw=1+2;
  // @formatter:on
  /* block */ + ;
  int after;
} }

class MalformedInsideIgnore { void m() {
  // @formatter:off
  +             ;
  // @formatter:on
  int after;
} }
