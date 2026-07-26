val annotated = { @Anno x: Int -> x }
val multiple = { @A @B x: Int, y: Int -> x }
val noType = { @Anno x -> x }
val plain = { x: Int -> x }
val destructured = { (a, b) -> a }
val softKeywordNames = { value -> value }
