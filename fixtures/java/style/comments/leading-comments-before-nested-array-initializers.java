class LeadingCommentsBeforeNestedArrayInitializers {
  String[][] sameLineFirst = new String[][] { /* dangling */ { "B3" }, { "B4" } };

  String[][] ownLineFirst =
    new String[][] {
      /* lead */
      { "B3" },
      { "B4" },
    };

  String[][] nonFirst = new String[][] {
    { "B3" },
    /* lead */
    { "B4" },
  };

  String[][] afterComma = new String[][] { { "B3" }, /* inline */ { "B4" } };

  String[][][] deeper = new String[][][] { /* dangling */ { { "B3" } }, { { "B4" } } };

  int[] scalars = { /* dangling */ 1, 2 };
}
