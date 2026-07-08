fun infixCalls(bits: Bits, mask: Bits): Bits {
    val combined = bits or mask and Bits.Enabled
    val readable = bits and mask or Bits.Enabled
    val rightOperand = bits context mask + Bits.Enabled
    val leftOperand = bits + mask context Bits.Enabled
    val named = combined context mask
    return named
}
