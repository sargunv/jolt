class Example {
void run(java.util.List<String> values) {
retry: for(String value:values) { if (stop(value)) break retry; process(value); }
while(ready()) ;
do processNext(); while(hasNext());
for(;;) ;
synchronized ( this ) { check(); }
}
}
