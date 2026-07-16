import z.B unexpected;
import a.A;

static transient class RecoveredModifiers {
    void parameter(final transient String value) {}

    void controls() {
        if () ;
        while () ;
    }
}
