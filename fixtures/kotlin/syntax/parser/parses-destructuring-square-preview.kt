fun squareDestructuring(record: Record, rows: List<Record>) {
    val [first, second, third] = record
    var [left, right] = record.bounds
    for ([name, value] in rows) {
        consume(first, second, third, left, right, name, value)
    }
}
