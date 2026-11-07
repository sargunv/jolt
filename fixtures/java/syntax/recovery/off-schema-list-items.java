public interface Foo {
  int other();
  helper(int x) { return; }
  default void third() { System.out.println("x"); }
}

interface I {
  default I() {}
}

@interface Note {
  Note(int a) {}
  int value();
}

class A {
  A() {
    this(a[], 2);
  }

  void m() {
    foo(a[], b);
    new X(a[], b);
    int[] o = {a[], b};
    m(1, 2, x.y[]);
  }
}
