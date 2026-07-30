package p;

import z.Zeta;

// Rides on a redundant separator that is removed, so it reads as the leading
// trivia of the import that follows. That import carries recovery and cannot
// prove it is sortable, so the comments stay a barrier ahead of it rather than
// travelling with it -- but they still lead it directly, so no blank line is
// invented between them, matching the sortable path.
;

import a.;

// Same, with the separator's comments and the import already adjacent.
;
import b.;

class ImportAfterRemovedSeparatorComments {}
