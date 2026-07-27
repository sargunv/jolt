class LambdaParameterListBreaking {
  void method(Handler handler) {
    handler.register((FirstEventTypeName firstEvent, SecondEventTypeName secondEvent) -> process(firstEvent, secondEvent));
  }
}
