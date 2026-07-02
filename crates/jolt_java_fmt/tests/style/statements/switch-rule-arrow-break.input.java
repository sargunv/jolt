class SwitchRuleArrowBreak {
String label(int value) {
return switch (value) {
case 1 -> "short";
case 2 -> { yield "block"; }
case String s when s.isBlank() -> "loooooooooooooooooooooooooooooooooooooooooooooooooooooooooong guarded expression";
case Pair(int left, _) -> "looooooooooooooooooooooooooooooooooooooooooooooooooooooong pattern expression";
default -> "looooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooooong expression";
};
}
}
