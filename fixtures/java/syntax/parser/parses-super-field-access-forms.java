class SuperFieldAccess extends Base {
    int field;

    class Inner extends Base {
        int method() {
            return super.field + SuperFieldAccess.super.field;
        }
    }
}

class Base {
    int field;
}
