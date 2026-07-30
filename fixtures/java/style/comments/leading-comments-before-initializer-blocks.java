class LeadingCommentsBeforeInitializerBlocks {
  /* instance lead */ {
    instance();
  }

  // instance line lead
  {
    lineLead();
  }

  /* static lead */ static {
    staticInit();
  }

  void nested() {
    /* block lead */ {
      nested();
    }
  }
}

class SameLineInstanceInitializerComment { /* dangling */ { body(); } }

class SameLineNestedBlockComment {
  void m() { /* dangling */ { body(); } }
}
