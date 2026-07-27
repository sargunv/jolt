fun inspectState(stateMachine: StateMachine): Int = when (val currentState = stateMachine.computeCurrentStateSnapshotForDiagnostics()) {
  else -> currentState.code
}
