fun compoundAssignment(counter: Counter, values: MutableList<Int>) {
    var total = 0
    total += counter.next()
    total -= 1
    total *= 2
    total /= 3
    total %= 4
    values[0] += total
}
