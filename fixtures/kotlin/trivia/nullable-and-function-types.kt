package com.example.trivia

typealias Callback =
    context(Logger /* JOLT-TRIVIA:context-function-type */) (String? /* JOLT-TRIVIA:nullable-parameter */) -> /* JOLT-TRIVIA:function-arrow */ Unit

interface Logger
