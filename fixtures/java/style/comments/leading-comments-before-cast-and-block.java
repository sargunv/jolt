class LeadingCommentsBeforeCastAndBlock {
  long cast(int value) {
    long result =
        // JOLT-TRIVIA:cast-line
        (long) value + 1;
    return result;
  }

  void block() {
    // JOLT-TRIVIA:block-line
    {
      int nested;
    }
    int following;
  }

  void blockComment() {
    /* JOLT-TRIVIA:block-comment */
    {
      int nested;
    }
  }
}
