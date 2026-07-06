package com.example.declarations

@Target(AnnotationTarget.CLASS, AnnotationTarget.FUNCTION)
annotation class Generated(val by: String)

@Generated("fixture") sealed class Result

data class Success(val value: String) : Result()
