class DocCommentStarLessLines {
    /**
     * A doc comment whose code sample has no leading stars:
     * <pre>{@code
     name.setTarget(constant("Rocky"));
     MethodHandle upcase = lookup()
         .findVirtual(String.class, "toUpperCase", type);
     }</pre>
     */
    void sample() {}

	/** for later:
	@Nested
	class NestedTests {

		@Test
		void test() {}
	}
	*/
	void tabs() {}

    /**
     * Initialize the IDs of fields that may be
       accessed from C.
     */
    void continued() {}
}
