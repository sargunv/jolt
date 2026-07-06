class Example {
  void run() {
    int value = compute(); // local value
    value++; // expression statement
    assert value > 0 : value; // assert detail
    if (value == 0) return; // return no value
    if (value == 1) return value /* returned value */; // return value
    if (value == 4) return // return line
    value;
    if (value == 2) break label; // break label
    if (value == 3) continue label; // continue label
    do value++; while (advance()); // do while
  }
}
