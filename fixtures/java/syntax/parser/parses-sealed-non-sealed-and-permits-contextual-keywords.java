sealed class SealedClass permits FinalClass, OpenClass {}
final class FinalClass extends SealedClass {}
non-sealed class OpenClass extends SealedClass {}

sealed interface SealedInterface permits OpenInterface {}
non-sealed interface OpenInterface extends SealedInterface {}
