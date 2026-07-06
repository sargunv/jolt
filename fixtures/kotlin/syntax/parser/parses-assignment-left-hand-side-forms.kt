fun assignmentTargets(target: Target, index: Int, value: String) {
    target.name = value
    target[index] = value
    target.child!!.name = value
    target.items[index].label = value
}
