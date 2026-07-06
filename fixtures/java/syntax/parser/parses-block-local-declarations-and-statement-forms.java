class Statements {
    void method(Object o) {
        ;
        class LocalClass {}
        interface LocalInterface {}
        final var local = o;
        label: if (local == null) ;
        if (local instanceof String s) s.trim(); else local.toString();
        assert local != null : local;
    }
}
