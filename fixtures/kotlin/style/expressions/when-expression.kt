package com.example.expressions

sealed interface Status {
    data object Ready : Status
    data class Failed(val reason: String) : Status
}

fun message(status: Status): String =
    when (status) {
        Status.Ready -> "ready"
        is Status.Failed -> "failed: ${status.reason}"
    }
