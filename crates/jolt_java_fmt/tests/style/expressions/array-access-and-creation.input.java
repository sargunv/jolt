class Example {
Object pick(Object[] values, int index) {
return values[ index ];
}
Object create(Object first, Object second) {
return new Outer<String>( ).new Inner(first,second);
}
int[] numbers() {
return new int[] {1,2,3};
}
String[] labels() {
return new String[] {"a very long label that forces the initializer to break across lines","another very long label that keeps each item on its own line","a third long label for the trailing comma policy"};
}
String[] localLabels() {
String[] labels = {"a very long local label that forces the initializer to break across lines","another very long local label that keeps each item on its own line","a third long local label for the trailing comma policy"};
return labels;
}
}
