# CPOP Instruction Optimization: AdvSIMD `cnt` on AArch64

## Summary

Replaced the scalar software popcount sequence in AArch64 `CPOP` and `CPOPW` handlers with an AdvSIMD-based reduction:

- `cnt` to count set bits per byte
- `addv` to horizontally sum 8 byte counts
- `umov` to move the final count back to a GPR

This removes a long integer instruction chain and constant loads from the hot path.

## Before / After

**Before** (scalar SWAR, many ops and literal loads):

```assembly
  mov RS2, RS1
  lsr RS2, RS2, 1
  ldr TEMP1, =0x5555555555555555
  and RS2, RS2, TEMP1
  sub RS1, RS1, RS2
  ...
  and RS1, RS1, 0x7f
```

**After** (SIMD, short straight-line sequence):

```assembly
  fmov d0, RS1
  cnt v0.8b, v0.8b
  addv b0, v0.8b
  umov RS1w, v0.b[0]
```

`cpopw` uses the same sequence after `mov RS1w, RS1w`, preserving RV64 `cpopw` semantics.

## Trade-offs

**Gains:**

- Much fewer instructions in `CPOP/CPOPW`
- No literal constant loads in handler

**Costs:**

- Depends on AdvSIMD availability on AArch64 targets
- Uses SIMD register `v0` in the interpreter loop

## Why This Is Safe

- `cnt v0.8b, v0.8b` computes popcount for each of the 8 bytes in `RS1`.
- `addv b0, v0.8b` sums those byte counts, yielding total set bits in the 64-bit value.
- `umov RS1w, v0.b[0]` returns the scalar count (`0..64`) to GPR for normal `WRITE_RD`.
- For `CPOPW`, zero-extending first ensures upper 32 bits do not contribute (`0..32`), matching RISC-V spec.
- While AdvSIMD (NEON) is an extension, it's mandatory on all AArch64 implementations, so this optimization should apply universally.

## Benchmark

### Assembly workload

[`tests/programs/cpop_microbench.S`](../tests/programs/cpop_microbench.S):

- 1,000,000 total operations
- Loop unrolled by 8
- Mixed sequence: 4x `cpop` + 4x `cpopw` per loop body

### Rust harness

[`tests/test_cpop_perf.rs`](../tests/test_cpop_perf.rs):

- 1 warm-up run + 5 timed runs
- Reports median/min/max wall-clock duration

Run:

```bash
cargo test --features=asm test_cpop_microbench -- --nocapture
```

## Correctness Coverage

- Existing functional test: `test_pcnt` in [`tests/test_b_extension.rs`](../tests/test_b_extension.rs), comparing `_rv64_pcnt/_rv32_pcnt` against software reference popcount.
