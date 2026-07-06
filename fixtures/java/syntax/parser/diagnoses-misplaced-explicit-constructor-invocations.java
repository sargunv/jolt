class MisplacedConstructorInvocations {
    MisplacedConstructorInvocations() {
        int value = 0;
        this();
        super();
    }
}
