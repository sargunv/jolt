class Example {
String escaped() {
return "line\\n" + '\t';
}
String block() {
return """
        alpha
          beta
        """;
}
int numeric() {
return 1_000 + 0x0f;
}
}
