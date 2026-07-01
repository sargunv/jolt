class Example {
  int run() {
    try {
      risky();
    } catch (java.io.IOException | RuntimeException ex) {
      return 1;
    } catch (Exception ex) {
      throw ex;
    } finally {
      cleanup();
    }
  }

  int recoverLong() {
    try {
      risky();
    } catch (
      com.example.recovery.FirstVeryLongRecoverableException
      | com.example.recovery.SecondVeryLongRecoverableException
      | com.example.recovery.ThirdVeryLongRecoverableException ex
    ) {
      return 2;
    }
  }

  int recoverAnnotated() {
    try {
      risky();
    } catch (@Nonnull final Exception ex) {
      return 3;
    }
  }
}
