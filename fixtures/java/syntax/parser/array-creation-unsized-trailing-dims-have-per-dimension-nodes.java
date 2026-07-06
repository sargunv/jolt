class ArrayCreationPerDimensionDims {
    void method() {
        Object values = new String[1] @A [] @B [];
    }
}

@interface A {}
@interface B {}
