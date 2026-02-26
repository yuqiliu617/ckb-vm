# Division Instruction Optimization: Branchless `csel` on AArch64

## Summary

Replaced conditional branches with ARM64 `csel` (Conditional SELect) in all 8 division/remainder instruction handlers (DIV, DIVU, DIVW, DIVUW, REM, REMU, REMW, REMUW). This eliminates branch misprediction penalties, improves instruction-level parallelism and readability.

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

`tests/test_division_perf.rs` — 1 warm-up run + 5 timed runs, reports median/min/max wall-clock time.

```bash
cargo test --features=asm test_division_microbench -- --nocapture
```

## RISC-V Division Edge Cases

| Condition    | `div`   | `divu`     | `rem`    | `remu`   |
| ------------ | ------- | ---------- | -------- | -------- |
| Divisor = 0  | -1      | 2^XLEN - 1 | dividend | dividend |
| INT_MIN / -1 | INT_MIN | N/A        | 0        | N/A      |
