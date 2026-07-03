# adversarial

google-java-format formatter test inputs.

## Metadata

| Tool        | Version       | Input files | Modified files |
| ----------- | ------------- | ----------: | -------------: |
| jolt        | jolt 0.0.0    |         207 |            195 |
| dprint-jolt | dprint 0.54.0 |         207 |            195 |

System: Linux 7.0.13-200.fc44.x86_64, x86_64, AMD RYZEN AI MAX+ 395 w/ Radeon
8060S, 32 logical CPUs, 125 GB memory. Hyperfine: 3 runs, 1 warmup.

```text
Benchmark 1: jolt fmt
  Time (mean ± σ):      14.3 ms ±   1.2 ms    [User: 62.3 ms, System: 24.3 ms]
  Range (min … max):    13.0 ms …  15.2 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):      24.1 ms ±   1.0 ms    [User: 72.0 ms, System: 50.7 ms]
  Range (min … max):    23.1 ms …  25.1 ms    3 runs
 
Summary
  jolt fmt ran
    1.68 ± 0.15 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
```
