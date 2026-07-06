fun invalidAssignmentTargets(value: Int) {
    1 = value
    value + 1 = value
    (value) = 2
    value() = 3
}
