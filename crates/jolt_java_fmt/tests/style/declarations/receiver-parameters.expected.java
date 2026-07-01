class ReceiverOuter {
  class ReceiverInner {
    ReceiverInner(@Readonly ReceiverOuter ReceiverOuter.this, int value) {
    }

    void bind(@Readonly ReceiverInner this, String value) {
    }
  }
}
