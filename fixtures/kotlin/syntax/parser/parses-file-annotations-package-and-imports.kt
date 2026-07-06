@file:FileMarker("parser")
@file:Suppress("unused")

package samples.structure

import kotlin.collections.List

class FilePreamble

@Target(AnnotationTarget.FILE)
annotation class FileMarker(val value: String)
