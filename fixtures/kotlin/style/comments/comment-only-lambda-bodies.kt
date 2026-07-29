fun commentOnlyLambdas() {
    inner {
        // do nothing
    }
    run({
        // wait for it
    })
    inner { /* marker */ }
    inner {
        /* multi
           line */
    }
}

fun commentMixedLambdas() {
    inner {
        doStuff()
        // trailing comment
    }
    inner { doStuff() }
}
