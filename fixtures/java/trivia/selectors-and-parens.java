class SelectorsAndParens {
  void run() {
    SelectorsAndParens value = this;
    value /* JOLT-TRIVIA:selector-target */ . child /* JOLT-TRIVIA:selector-name-before-call */ ()
        . child /* JOLT-TRIVIA:second-selector-name-before-call */ ()
        . field /* JOLT-TRIVIA:field-select-before-equals */ = value /* JOLT-TRIVIA:field-select-before-semi */;
    String s =
        (( /* JOLT-TRIVIA:outer-paren-open */ String /* JOLT-TRIVIA:cast-type-before-close */)
                /* JOLT-TRIVIA:after-cast-paren */ ( /* JOLT-TRIVIA:inner-paren-open */ Object /* JOLT-TRIVIA:inner-type-name */) "x")
            /* JOLT-TRIVIA:cast-expression-before-semi */;
    Object[] values =
        new Object /* JOLT-TRIVIA:array-type-before-empty-dims */ [] /* JOLT-TRIVIA:array-empty-dims-before-init */ {
          value /* JOLT-TRIVIA:array-init-element */
        };
  }

  SelectorsAndParens child() {
    return this;
  }

  SelectorsAndParens field;
}
