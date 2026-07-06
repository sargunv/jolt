class BinaryPrecedence {
    void method(int a, int b, int c) {
        boolean logical = a < b && b < c || c == a;
        int arithmetic = a + b * c - a / b % c;
        int left = a << 1;
        int right = b >> 1;
        int unsigned = c >>> 1;
        int bits = a | b ^ c & a;
    }
}
