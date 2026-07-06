interface A {}
interface B {}

class IntersectionTypeShapes<T extends A & B> {
    Object method(Object value) {
        return (A & B) value;
    }
}
