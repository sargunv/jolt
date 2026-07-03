# realistic

Spring Framework Java sources.

## Metadata

| Tool        | Version       | Input files | Modified files |
| ----------- | ------------- | ----------: | -------------: |
| jolt        | jolt 0.0.0    |        9136 |           8620 |
| dprint-jolt | dprint 0.54.0 |        9136 |           8702 |

System: Linux 7.0.13-200.fc44.x86_64, x86_64, AMD RYZEN AI MAX+ 395 w/ Radeon
8060S, 32 logical CPUs, 125 GB memory. Hyperfine: 3 runs, 1 warmup.

```text
Benchmark 1: jolt fmt
  Time (mean ± σ):     873.1 ms ±  21.6 ms    [User: 16774.7 ms, System: 784.5 ms]
  Range (min … max):   851.3 ms … 894.4 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):     837.1 ms ±   3.6 ms    [User: 17232.2 ms, System: 1093.1 ms]
  Range (min … max):   833.1 ms … 839.8 ms    3 runs
 
Summary
  dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format ran
    1.04 ± 0.03 times faster than jolt fmt
```
