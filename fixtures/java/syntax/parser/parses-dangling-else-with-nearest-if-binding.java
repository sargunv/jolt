class DanglingElse {
    void method(boolean first, boolean second) {
        if (first)
            if (second)
                winner();
            else
                loser();
    }

    void winner() {}
    void loser() {}
}
