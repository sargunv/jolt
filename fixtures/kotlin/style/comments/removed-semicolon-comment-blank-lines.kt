val trailingOnly = 1; // trailing comment salvaged from the removed semicolon

val leadingRun = 2
// first comment salvaged from the removed semicolon

// second salvaged comment after a blank line
; // trailing comment salvaged after the leading run
val afterSalvage = 3

val collapsedRun = 4
// salvaged comment before collapsed blank lines



// salvaged comment after collapsed blank lines
;
val last = 5
