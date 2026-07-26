class DanglingCommentBlankLines {
  // first dangling comment of the class body

  // second dangling comment after a blank line



  // third dangling comment after collapsed blank lines
  // an adjacent comment with no blank line before it
}

class DanglingCommentsInBodies {
  void emptyBlock() {
    // first dangling comment of the block

    // second dangling comment after a blank line
  }

  void emptyArguments() {
    call(
      // first dangling comment of an empty argument list

      // second dangling comment after a blank line
    );
  }

  void call() {}
}

class RemovedSemicolonComments {
  ;
  // a comment salvaged from a removed semicolon

  // a second salvaged comment
  ;
}
