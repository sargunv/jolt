fun adjacentControlFlowWithoutTerminator() {
  for (i in 1..2) { consume(i) } while (ready()) { consume(next()) } /* recovery boundary */
}
