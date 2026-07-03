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
  Time (mean ± σ):      11.7 ms ±   1.5 ms    [User: 55.6 ms, System: 24.2 ms]
  Range (min … max):    10.3 ms …  13.3 ms    3 runs
 
Benchmark 2: dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
  Time (mean ± σ):      24.8 ms ±   2.2 ms    [User: 77.8 ms, System: 41.6 ms]
  Range (min … max):    23.4 ms …  27.4 ms    3 runs
 
Summary
  jolt fmt ran
    2.12 ± 0.33 times faster than dprint --plugins=jolt_fmt_dprint.wasm fmt --incremental=false --skip-stable-format
```
