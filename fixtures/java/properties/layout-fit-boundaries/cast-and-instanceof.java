class CastAndInstanceofLayoutFitBoundaries {
  void layout(Object someObjectReference) {
    Object value = (SomeVeryLongCastTargetTypeName) someExpressionThatIsAlsoQuiteLongHere.getValue();
    boolean matches = someObjectReference instanceof SomeVeryLongPatternTypeName someBindingName;
  }
}
