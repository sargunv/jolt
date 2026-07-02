class Example {
  int nestedTernary(int i) {
    int j =
      i == 0
        ? 0
        : i == 1
          ? 1
          : i == 2
            ? 2
            : i == 3
              ? 3
              : i == 4
                ? 4
                : i == 5
                  ? 5
                  : i == 6
                    ? 6
                    : i == 7
                      ? 7
                      : i;
    return j;
  }
}
