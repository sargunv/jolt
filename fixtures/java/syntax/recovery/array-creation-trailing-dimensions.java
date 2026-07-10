class ArrayCreationTrailingDimensions {
    Object value = new String[1] @First [] @Second [];
}

@interface First {}
@interface Second {}
