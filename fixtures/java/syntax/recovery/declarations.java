enum RecoveredEnum {
    ONLY,
    @Marker() /* JOLT-TRIVIA:enum-annotation */
    (/* JOLT-TRIVIA:enum-arguments */ 1) {
        /* JOLT-TRIVIA:enum-body */
        void recovered() {}
    },
    /* JOLT-TRIVIA:enum-recovered */ ,
    ;
    int member;
}

enum RecoveredSeparatorEnum {
    ONLY /* JOLT-TRIVIA:enum-separator */ ;
    int member;
}
