package com.example.expressions

val normalized=" Alpha Beta Gamma ".trim().lowercase().replace(" ","_").removePrefix("tmp_").removeSuffix("_old")

fun canonical(input: String): String=input.trim().lowercase().replace("-","_")
