class ArrayInitializerCommaComments {
  int[] values = {
    2, /* JOLT-TRIVIA:trailing-two */
    // JOLT-TRIVIA:leading-three
    3,
  };
}
