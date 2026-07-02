class Runner {
  public Runner() throws IOException, TimeoutException {
  }

  public Runner(Request request, ExecutionContext executionContext)
    throws /* ctor */ IOException, // io
      TimeoutException {
  }

  public Result compute(
    Request request,
    ExecutionContext executionContext,
    RetryPolicy retryPolicy
  )
    throws /* checked */ IOException, // io
      TimeoutException {
  }
}
