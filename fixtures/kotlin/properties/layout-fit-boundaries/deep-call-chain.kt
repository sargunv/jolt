package com.example.properties

val normalized =
    " Alpha-Beta Gamma "
        .trim()
        .lowercase()
        .replace(" ", "_")
        .removePrefix("tmp_")
        .removeSuffix("_old")
