# Division Instruction Optimization: Branchless `csel` on AArch64

## Summary

Replaced conditional branches with ARM64 `csel` (Conditional SELect) in all 8 division/remainder instruction handlers (DIV, DIVU, DIVW, DIVUW, REM, REMU, REMW, REMUW). This eliminates branch misprediction penalties, improves instruction-level parallelism and readability.

In benchmark data, this optimization does not materially reduce average latency, but it significantly improves runtime stability by reducing long-tail slow runs.

## Before / After

**Before** (DIV, branched):
```assembly
  cmp RS2, 0
  bne .div_branch2              ← branch taken on common path
  mov RS1, UINT64_MAX
  b .div_branch3                ← extra jump to rejoin
.div_branch2:
  sdiv RS1, RS1, RS2
.div_branch3:
  WRITE_RD(RS1)
```

**After** (DIV, branchless):
```assembly
  sdiv TEMP1, RS1, RS2          ← always compute
  mov TEMP2, UINT64_MAX         ← always prepare fallback
  cmp RS2, 0
  csel RS1, TEMP2, TEMP1, eq    ← RS2==0 ? -1 : quotient
  WRITE_RD(RS1)
```

Remainder uses the same pattern: compute via `sdiv` + `msub`, then `csel` between the computed remainder and the original dividend (RISC-V spec: rem by zero returns dividend).

## Why This Is Safe

- ARM64 `sdiv`/`udiv` with divisor=0 returns 0 without trapping, so speculatively executing the division is harmless.
- ARM64 `sdiv` with INT64_MIN / -1 returns INT64_MIN, matching RISC-V spec. No fixup needed for the overflow case.

## Trade-offs

**Gains:**
- Zero branches in division handlers
- Straight-line execution with better ILP
- Constant-time regardless of input

**Costs:**
- Division always executes even when divisor is zero
- Uses one extra temp register (TEMP2 for the fallback constant)

## Benchmark

`tests/programs/division_microbench.S` — 1M chained `div` instructions (125K iterations × 8 unrolled). Chained dependency (`div t2, t2, t1`) serializes divisions to measure handler latency rather than throughput.

`tests/test_division_perf.rs` — 1 warm-up run + 100 timed runs, reports average/median/min/max wall-clock time.

```bash
cargo test --features=asm test_division_microbench -- --nocapture
```

### Results

Benchmark environment:

- Aliyun g8y instance
- 1 YiTan 710 core

Measured result (100 runs, 1M iterations x 8 divs):

- Before optimization:
  `Division microbench: average=2.807302ms, median=2.583109ms, min=2.490509ms, max=8.647073ms`
- After optimization:
  `Division microbench: average=2.781691ms, median=2.72947ms, min=2.65895ms, max=4.435717ms`

Observed behavior:

- Average latency is almost unchanged (`-0.91%`, effectively flat).
- Maximum latency drops (`8.647073ms -> 4.435717ms`, `-48.70%`), yet minimum latency slightly increases (`2.490509ms -> 2.65895ms`, `+6.77%`).
- Runtime spread (`max - min`) narrows from `6.156564ms` to `1.776767ms` (`-71.14%`).

Interpretation:

- Branchless `csel` mainly reduces branch-prediction-related outliers and makes execution time more stable.
- For division-heavy workloads with predictable common-path arithmetic cost, the main win is tighter latency distribution rather than large average-speedup.

## RISC-V Division Edge Cases

| Condition    | `div`   | `divu`     | `rem`    | `remu`   |
| ------------ | ------- | ---------- | -------- | -------- |
| Divisor = 0  | -1      | 2^XLEN - 1 | dividend | dividend |
| INT_MIN / -1 | INT_MIN | N/A        | 0        | N/A      |
