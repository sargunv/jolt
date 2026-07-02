class Example {
  void values() {
    Object[] valuesThatForceTheArrayInitializerOntoTheNextLineHere =
      new Object[] {
        new Object(),
        new Object(),
      };
  }

  void inlineValues() {
    Object[] values = new Object[] { new Object(), new Object() };
  }
}
