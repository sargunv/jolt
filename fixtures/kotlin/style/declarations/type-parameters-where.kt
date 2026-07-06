package com.example.declarations

interface Source<T>
interface Sink<T>

fun <T, R> connect(source: T, value: R): R
    where T : Source<R>,
          T : Sink<R> {
    return value
}
