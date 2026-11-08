package p;

import z.Zeta;

// A comment on a redundant separator keeps the separator, which then splits
// the imports. The import after it carries recovery and cannot prove it is
// sortable, so it stays in place.
;

import a.;

// Same, with the separator and the import already adjacent.
;
import b.;

class ImportAfterCommentedSeparator {}
