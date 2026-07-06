fun infixCalls(bits: Bits, mask: Bits): Bits {
    val combined = bits or mask and Bits.Enabled
    val named = combined context mask
    return named
}
