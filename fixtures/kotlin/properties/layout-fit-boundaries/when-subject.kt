fun shortSubject(value: State): Int = when (val current = value) {
  else -> current.code
}

fun typedSubject(value: State): Int = when (val current: State = value) {
  else -> current.code
}

fun inspectState(stateMachine: StateMachine): Int = when (val currentState = stateMachine.computeCurrentStateSnapshotForDiagnostics()) {
  else -> currentState.code
}

fun commentedSubject(stateMachine: StateMachine): Int = when (val currentState = stateMachine.computeCurrentStateSnapshotForDiagnostics() // computed subject
) {
  else -> currentState.code
}

fun typedCommentedSubject(stateMachine: StateMachine): Int = when (val currentState: State /* subject type */ = stateMachine.computeCurrentStateSnapshotForDiagnostics()) {
  else -> currentState.code
}

fun delimiterComments(stateMachine: StateMachine): Int = when ( // after open
  val currentState = stateMachine.computeCurrentStateSnapshotForDiagnostics()
  // before close
) {
  else -> currentState.code
}
