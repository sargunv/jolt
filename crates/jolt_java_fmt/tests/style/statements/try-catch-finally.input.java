class Example {
int run() {
try { risky(); } catch (java.io.IOException | RuntimeException ex) { return 1; } catch(Exception ex) { throw ex; } finally { cleanup(); }
}
}
