class Example {
void run() throws Exception {
try (var declared = open(); existing;) { use(declared); } catch (Exception ex) { recover(ex); } finally { cleanup(); }
}
AutoCloseable open() { return null; }
AutoCloseable existing;
}
