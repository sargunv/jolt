package com.example.imports

import com.example.tools.Factory as ToolFactory
import com.example.tools.*
import kotlin.io.println as say

fun aliases(factory: ToolFactory) {
    say(factory.toString())
}
