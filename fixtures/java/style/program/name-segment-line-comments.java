package a //x
;

import b.c.D //y
;

import e.F //z
;

class NameSegmentLineComments {
  void method(
    final @Deprecated // annotation stays put
    String value
  ) {
  }
}
