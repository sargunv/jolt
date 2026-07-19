class CommentLexicalBoundaries {
int/**/field=0;
java.util.List/**/<String/**/> names;

int operators(int left,int right) {
int plus=left+/**/+right;
int minus=left-/**/-right;
return/**/plus/**/+/**/minus;
}

void delimiters() {
consume/**/(/**/"value"/**/);
this/**/.field=1;
}

void adjacent() {
/*alpha*//**//*beta*/
/****/
/*****/
/*/*/
/* **/
/* ***/
}

void consume(Object value) {}
}