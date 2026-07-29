class Unterminated {
    void inMethod() {
        int before = 1;
        // @formatter:off
        int kept   =   2  +  3;
        int also   =   4;
    }

    int after   =   5;
}
