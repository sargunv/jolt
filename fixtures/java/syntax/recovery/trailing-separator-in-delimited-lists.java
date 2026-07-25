class TrailingSeparatorInDelimitedLists<T,> {
java.util.List<String,> typeArguments;
void parameters(int a,) {}
void varargsParameters(int... values,) {}
void arguments() { consume(1,); }
void throwsClause() throws E, {}
void forUpdate() { for (int i = 0;; i++,) {} }
void recordPattern(Object o) { if (o instanceof R(int a,)) {} }
void lambdaParameters() { consume((a,) -> a); }
@A(value = 1,) int annotationArguments;
void consume(Object... values) {}
}

interface ExtendsList extends A, {}

class ImplementsList implements A, {}

sealed class PermitsList permits A, {}

record RecordHeader(int a,) {}

enum ValidTrailingSeparators { A, }

class ValidTrailingSeparatorInArrayInitializer { int[] values = {1,}; }

class TrailingSeparatorAtEof { void m() { consume(1,
