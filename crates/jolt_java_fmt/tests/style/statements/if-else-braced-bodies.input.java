class Example {
  void run() { if (/* guard */ ready) run(); else if (waiting) pause(); else /* fallback */ stop(); }
  void comments() {
    if (ready) { run(); } /* after then */ else stop();
    if (waiting) { pause(); } // after wait
    else resume();
  }
}
