class LeadingCommentsOnBinaryOperands {
  int compute(int first, int second, int third) {
    int sum =
      // leads the left operand of a binary chain
      first + second;
    int product =
      /* block comment leads the left operand */
      first * second * third;
    boolean flag =
      // leads an operand of a mixed chain
      first > second && second < third;
    int total =
      // an operand chain that does not fit still breaks at its operators
      first + second + third + sum + product + first + second + third + sum + product;
    return flag ? total : sum;
  }
}
