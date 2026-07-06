enum Planet {
    MERCURY(1),
    VENUS(2) {
        void override() {}
    };

    final int order;

    Planet(int order) {
        this.order = order;
    }

    void override() {}
}
