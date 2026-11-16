# What does `sigsetjmp` on entry cost?

This benchmark
measures the cost of setjmp per call into wasm.

**Result: about 5 ns (16 cycles) per exported call** on Linux x86-64.

## What is measured

`exports.cc` holds three versions of one exported function, `x -> x + 1`. They
differ only in what happens on entry. Each is what `wasm_entry` in
[`src/runtime.hh`](../src/runtime.hh) does in one of its configurations:

| Variant  | On entry                                             | Configuration                              |
|----------|------------------------------------------------------|--------------------------------------------|
| `plain`  | nothing                                              | `W2CC_GUARD_PAGES=0`                       |
| `unwind` | one call to a wrapper that is never inlined          | `W2CC_GUARD_PAGES=1`, `W2CC_TRAP_UNWIND=1` |
| `setjmp` | `sigsetjmp(env, 0)` and a thread-local jump target   | `W2CC_GUARD_PAGES=1`, `W2CC_TRAP_UNWIND=0` |

`main.cc` is the host. It calls each export 20 million times and divides the
elapsed time by 20 million. It does that 25 times per variant, taking turns
between variants, and prints the fastest and the median round.

## Running it

```sh
bench/run.sh 254     # 254: the CPU to pin to
```

Needs `g++` and `clang++` with C++23, and `taskset`.

## Results

AMD EPYC 7702 (shared server, performance governor), Linux 7.0, glibc 2.43,
`-O2`, pinned to an idle CPU.

| Variant  | g++ 15.2: ns | cycles | instructions | clang++ 21.1: ns | cycles | instructions |
|----------|-------------:|-------:|-------------:|-----------------:|-------:|-------------:|
| `plain`  |         1.50 |      5 |            7 |             1.50 |      5 |            6 |
| `unwind` |         2.70 |      9 |           14 |             3.00 |     10 |           16 |
| `setjmp` |         6.60 |     22 |           74 |             6.30 |     21 |           62 |

- `setjmp` costs 4.8 to 5.1 ns more than `plain`, and 3.3 to 3.9 ns more than
  `unwind`.

## Why the numbers can be trusted

- **The calls happen.** The exports are in a separate file, compiled without
  LTO, so the compiler cannot inline or remove them. Each call's argument is
  the previous call's result, and the final result is checked to equal the
  number of calls.
- **Noise is controlled.** The process is pinned to one CPU. Variants take
  turns, so a disturbance hits all of them. The fastest round is the one with
  the least interference, and it agrees with the median.
- **There is a second measurement.** `perf` counts cycles and instructions per
  call. Those do not depend on the clock or on the CPU frequency, and they
  agree with the times (about 0.3 ns per cycle).
- **You can read what ran.**
  `objdump -dC bench/build/entry_cost | less`, then search for
  `call_repeatedly`: each loop is one `call` to its export and a counter.
  `export_setjmp` contains the call to `__sigsetjmp`.
