class IntegerLiteralBoundaries {
    void method() {
        int min = -2_147_483_648;
        int badInt = 2_147_483_648;
        int badParenthesizedInt = -(2147483648);
        long minLong = -9_223_372_036_854_775_808L;
        long badLong = 9_223_372_036_854_775_808L;
        long badPlusLong = +9223372036854775808L;
        switch (badInt) {
            case -2147483648:
                break;
            case 2147483648:
                break;
        }
    }
}
