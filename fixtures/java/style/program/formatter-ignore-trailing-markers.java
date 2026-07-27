class FormatterIgnoreTrailingMarkers {
  void  before(){ int value= 0; } // @formatter:off
  void  inside(){ int value= 0; } // @formatter:on
}
