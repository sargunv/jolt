class Example {
void run() {
int before=1;
// @formatter:off
int first=1+2;
// @formatter:on
// @formatter:off
int second=3+4;
// @formatter:on
int after=5;
}

void repeatedOff() {
int before=1;
// @formatter:off
int first=1+2;
// @formatter:off
int second=3+4;
// @formatter:on
int after=5;
}

void sameLineTransition() {
int before=1;
// @formatter:off
int first=1+2;
/* @formatter:on */ /* @formatter:off */
int second=3+4;
// @formatter:on
int after=5;
}
}
