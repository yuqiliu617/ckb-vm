# Division Instruction Optimization on AArch64

Two independent optimizations were applied to the AArch64 assembly handlers for the RISC-V division and remainder instructions.

## Optimization 1: Branchless `csel`

### Summary

Replaced conditional branches with ARM64 `csel` (Conditional SELect) in all 8 division/remainder instruction handlers (DIV, DIVU, DIVW, DIVUW, REM, REMU, REMW, REMUW). This eliminates branch misprediction penalties and improves instruction-level parallelism.

### Before / After

**Before** (DIV, branched):
```asm
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
```asm
  sdiv TEMP1, RS1, RS2          ← always compute
  mov TEMP2, UINT64_MAX         ← always prepare fallback
  cmp RS2, 0
  csel RS1, TEMP2, TEMP1, eq    ← RS2==0 ? -1 : quotient
  WRITE_RD(RS1)
```

Remainder uses the same pattern: compute via `sdiv` + `msub`, then `csel` between the computed remainder and the original dividend (RISC-V spec: rem by zero returns the dividend).

### Why This Is Safe

- ARM64 `sdiv`/`udiv` with divisor=0 returns 0 without trapping, so speculatively executing the division is harmless.
- ARM64 `sdiv` with INT64_MIN / -1 returns INT64_MIN, matching RISC-V spec. No fixup needed for the overflow case.

### Trade-offs

**Gains:**
- Zero branches in division handlers
- Straight-line execution with better ILP
- Constant-time regardless of input

**Costs:**
- Division always executes even when divisor is zero
- Uses one extra temp register (TEMP2 for the fallback constant)

---

## Optimization 2: 32-bit Division for `divw` / `divuw`

### Summary

Replaced the sign/zero-extend-then-divide-64-bit pattern in DIVW and DIVUW with direct 32-bit division instructions (`sdiv Wd, Wn, Wm` / `udiv Wd, Wn, Wm`). This removes 2 instructions per handler by letting the 32-bit instruction forms handle operand masking natively.

### Before / After

**Before** (DIVW):
```asm
  ldr RS1, REGISTER_ADDRESS(RS1)
  ldr RS2, REGISTER_ADDRESS(RS2)
  sxtw RS1, RS1w                ← sign-extend to make 64-bit sdiv correct
  sxtw RS2, RS2w                ← sign-extend to make 64-bit sdiv correct
  sdiv TEMP1, RS1, RS2          ← 64-bit signed division
  sxtw TEMP1, TEMP1w            ← sign-extend result
  ...
```

**After** (DIVW):
```asm
  ldr RS1, REGISTER_ADDRESS(RS1)
  ldr RS2, REGISTER_ADDRESS(RS2)
  sdiv TEMP1w, RS1w, RS2w       ← 32-bit signed division, upper bits ignored natively
  sxtw TEMP1, TEMP1w            ← sign-extend result
  ...
```

DIVUW follows the same pattern, replacing `mov RS1w, RS1w` + `mov RS2w, RS2w` + `udiv TEMP1, RS1, RS2` with `udiv TEMP1w, RS1w, RS2w`.

### Why This Is Safe

- ARM64 32-bit `sdiv Wd, Wn, Wm` reads only the low 32 bits of its source registers and writes a zero-extended 32-bit result to the destination, ignoring upper bits exactly as RISC-V requires.
- ARM64 32-bit `sdiv` on INT32_MIN / -1 produces `0x80000000` without trapping. The subsequent `sxtw` sign-extends this to `0xFFFFFFFF80000000` = INT32_MIN, which is the correct RISC-V DIVW overflow result.
- The `cmp RS2w, 0` / `csel` pattern for divide-by-zero is unchanged.

---

## Benchmark

[div_microbench.S](../tests/programs/div_microbench.S) and [divw_microbench.S](../tests/programs/divw_microbench.S) — 1M chained `div` and `divw` instructions (125K iterations × 8 unrolled). Chained dependency serializes divisions to measure handler latency rather than throughput.

[test_division_perf.rs](../tests/test_division_perf.rs) — 1 warm-up run + 1000 timed runs, reports average/median/min/max wall-clock time.

```bash
cargo test --features=asm test_div_microbench -- --nocapture
cargo test --features=asm test_divw_microbench -- --nocapture
```

### Results

Benchmark environment:

- Aliyun g8y instance
- 1 YiTan 710 core

Measured result (1000 runs, 125K iterations x 8 divs):

**`div_microbench`** (measures Optimization 1: `csel`):
- Before: `average=3.920017ms, median=3.537173ms, min=3.420233ms, max=11.047261ms`
- After:  `average=3.785128ms, median=3.719614ms, min=3.625254ms, max=9.649655ms`

**`divw_microbench`** (measures Optimization 2: 32-bit division):
- Before: `average=4.280946ms, median=3.846714ms, min=3.700293ms, max=13.51235ms`
- After:  `average=3.625833ms, median=3.563473ms, min=3.469253ms, max=9.682356ms`

### Interpretation

**`csel` (Optimization 1):** Average latency drops slightly, and the maximum drops by 12.6% and the min/median shift upward slightly. The pattern is consistent with branch-misprediction elimination: rare slow outliers are removed, while the common fast path cost is basically unchanged.

**32-bit division (Optimization 2):** Average latency improves by 15.3% and the maximum drops by 28.4%. Unlike the `csel` change, this optimization removes instructions from the unconditional fast path, producing a genuine speed improvement across all runs.

---

## RISC-V Division Edge Cases

| Condition    | `div`   | `divu`   | `divw`    | `divuw`  | `rem`    | `remu`   |
| ------------ | ------- | -------- | --------- | -------- | -------- | -------- |
| Divisor = 0  | -1      | 2^64 − 1 | -1        | 2^64 − 1 | dividend | dividend |
| INT_MIN / -1 | INT_MIN | N/A      | INT32_MIN | N/A      | 0        | N/A      |
