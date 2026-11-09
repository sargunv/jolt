class DocLineCommentRuns
/// Before the body brace: moves into the body, where a blank line keeps it
/// apart from the run of the first member.
{
  /// The first member.
  void first() {}

  /// Before a removed semicolon.
  ;
  /// The second member.
  void second() {}

  /// A run
  //// ends at a line of four slashes,
  /// and starts again after it.
  int third;

  int fourth; /// A doc line after code starts a run, as javac reads it,
  /// so the line after it stays right after it.
  int fifth;

  int sixth // before the semicolon
  ; /// After code, the first line of the run below; placement moves it to its own line.
  /// The seventh member.
  int seventh;
}
