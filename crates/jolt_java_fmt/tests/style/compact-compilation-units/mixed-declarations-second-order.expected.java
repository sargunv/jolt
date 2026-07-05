package demo;

import java.util.Map;

void main() {
  Runner.run();
}

record Point(int x, int y) {
}

static final Map<String, Integer> COUNTS = Map.of("one", 1);

class Runner {
  static void run() {
    IO.println(COUNTS);
  }
}

interface Job {
  void run();
}

enum Status {
  READY,
}

@interface Tag {
}
