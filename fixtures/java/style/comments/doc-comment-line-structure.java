class DocCommentLineStructure {
  /**
   * Interceptor callbacks, in order:
   * 1) beforeConcurrentHandling, before the task is submitted for execution
   * 2) preProcess, once the task is running
   * - postProcess, after it completes, which is long enough that it does not fit on one line
   * - afterCompletion
   * • a bullet
   */
  void plainTextList() {}

  /**
   * The graph:
   *      1
   *     / \
   *    2   3
   */
  void asciiGraph() {}

  /**
   * @param grammar the grammar:
   *     expr ::= term
   *            | expr '+' term
   */
  void grammar(String grammar) {}

  /**
   * Rulers keep their lines:
   * -------
   * Header row
   * |-----|
   * The end.
   */
  void rulers() {}

  /**
   * A method.
   *
   * @param x a value, one of:
   * - first item
   * - second item
   */
  void listInATag(int x) {}

  /**
   * A method.
   *
   * @param x heading
   * -----
   * text under the ruler
   */
  void rulerInATag(int x) {}
}
