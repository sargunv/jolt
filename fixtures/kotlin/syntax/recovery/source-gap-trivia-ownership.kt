fun triviaOwnedBlockGaps() {
  val first = 1

  /* JOLT-TRIVIA:blank-line-before-comment */
  val second = 2
  /* JOLT-TRIVIA:comment-resets-newline-run */
  val third = 3 // JOLT-TRIVIA:line-comment-no-blank
  val fourth = 4 // JOLT-TRIVIA:line-comment-before-blank

  val fifth = 5
}

val firstTopLevel = 1 // JOLT-TRIVIA:top-level-line-comment-no-blank
val secondTopLevel = 2 // JOLT-TRIVIA:top-level-line-comment-before-blank

val thirdTopLevel = 3
