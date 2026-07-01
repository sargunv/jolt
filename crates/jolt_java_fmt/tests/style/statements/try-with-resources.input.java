class Example {
void run() throws Exception {
try (var declared = open(); existing;) { use(declared); } catch (Exception ex) { recover(ex); } finally { cleanup(); }
}
void annotated() throws Exception {
try (@Nonnull AutoCloseable declared = open(); this.existing) { use(declared); }
}
AutoCloseable open() { return null; }
AutoCloseable existing;
}
