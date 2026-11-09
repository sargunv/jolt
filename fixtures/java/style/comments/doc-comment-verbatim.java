class DocCommentVerbatim {
  /**
   * Tabs after the star stay.
   * <pre class="code">
   *	client.get()
   *		.retrieve();
   * </pre>
   * A ruler in code:
   * <pre>
   *=====
   *   x  
   *=====
   * </pre>
   */
  void tabsAndRulers() {}

  /**
   * <ul>
   * <li>An item with code:
   * <pre>
   *   run();
   * </pre>
   * </li>
   * </ul>
   * @param task the task, for example
   * <pre>{@code
   *   Map<String, List<Integer>> map = new HashMap<>();
   * }</pre>
   * @return the result
   */
  int nested(Runnable task) { return 0; }

  /**
   * Upper case:
   * <PRE>
   *   SELECT * FROM t
   * </PRE>
   * A long code line overflows:
   * <pre>
   * someObject.someMethodWithAVeryLongName(firstArgument, secondArgument, thirdArgument);
   * @Bean starts a line in code, which javac reads as a block tag even in pre.
   * </pre>
   */
  void upperCaseAndLong() {}

  /**
   * A snippet:
   * {@snippet lang=java :
   *   if (v.isPresent()) {
   *     System.out.println("v: " + v.get()); // @highlight substring="println"
   *   }
   * }
   * A snippet on one line: {@snippet : run(); }
   */
  void snippet() {}

  /**
   * An unclosed pre runs to the end:
   * <pre>
   *   run();
   *       more();
   */
  void unclosed() {}
}
