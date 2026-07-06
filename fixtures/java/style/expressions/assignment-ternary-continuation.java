class Example {
Object choose(Object candidate) {
Object selectedValueWithNameLongEnoughToBreak = candidate != null ? candidate : new Object();
return selectedValueWithNameLongEnoughToBreak;
}
}
