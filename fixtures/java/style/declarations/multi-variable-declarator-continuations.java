class MultiVariableDeclaratorContinuations {
  int firstFieldVariable = 1, secondFieldVariable = 2, thirdFieldVariable = provider();

  void method() {
    int firstLocalVariable = 1, secondLocalVariable = 2, thirdLocalVariable = provider();
  }

  int provider() {
    return 3;
  }
}
