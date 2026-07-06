class QualifiedThis {
    class Inner {
        Object method() {
            return QualifiedThis.this;
        }
    }
}
