class Example {
int run(Object value) {
assert value != null : value;
if (value == null) throw /* problem */ new IllegalArgumentException();
while(ready()) continue retry;
return /* result */ 1;
}
int choose(int value) {
return switch (value) { case 1 -> 1; default -> { yield /* fallback */ 2; } };
}
}
