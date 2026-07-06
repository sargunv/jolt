/* JOLT-TRIVIA:file-header */
@file:Suppress("unused") /* JOLT-TRIVIA:file-annotation */
package com /* JOLT-TRIVIA:package-mid */ .example.trivia // JOLT-TRIVIA:package-tail

// JOLT-TRIVIA:before-imports
import kotlin.collections /* JOLT-TRIVIA:import-dot */ .List
import kotlin.io.println /* JOLT-TRIVIA:import-alias-source */ as say
import com.example /* JOLT-TRIVIA:star-prefix */ .tools.*
// JOLT-TRIVIA:after-imports

fun importedNames(names: List<String>) {
    say(names.joinToString())
}
