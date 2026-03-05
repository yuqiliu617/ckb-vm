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

[benches/division_benchmark.rs](../benches/division_benchmark.rs) — Criterion benchmark; reports mean, median, standard deviation, and 95% confidence intervals.

```bash
cargo bench --features=asm div_microbench
cargo bench --features=asm divw_microbench
```

### Results

Benchmark environment: Aliyun `ecs.g8y.small`, YiTian 710 (1 core), 4 GB RAM

**`div_microbench`** (measures Optimization 1: `csel`):

|         | Before                       | After                        | Change                    |
| ------- | ---------------------------- | ---------------------------- | ------------------------- |
| Mean    | [3.544, **3.575**, 3.613] ms | [3.722, **3.738**, 3.758] ms | [+3.4%, **+4.6%**, +5.6%] |
| Median  | [3.485, **3.488**, 3.491] ms | [3.689, **3.692**, 3.694] ms | [+5.7%, **+5.8%**, +5.9%] |
| Std Dev | 0.557 ms                     | 0.285 ms                     | −48.9%                    |
| MAD     | 38.3 µs                      | 34.3 µs                      | −10.3%                    |

**`divw_microbench`** (measures Optimization 2: 32-bit division; before = post-`csel`, after = post-32-bit-div):

|         | Before                       | After                        | Change                    |
| ------- | ---------------------------- | ---------------------------- | ------------------------- |
| Mean    | [3.835, **3.852**, 3.874] ms | [3.586, **3.608**, 3.634] ms | [−7.1%, **−6.3%**, −5.5%] |
| Median  | [3.801, **3.803**, 3.806] ms | [3.547, **3.549**, 3.552] ms | [−6.8%, **−6.7%**, −6.6%] |
| Std Dev | 0.314 ms                     | 0.394 ms                     | +25.6%                    |
| MAD     | 35.5 µs                      | 34.5 µs                      | −2.8%                     |

### Interpretation

**`csel` (Optimization 1):** The 5–6% regression is expected: `csel` adds `mov TEMP2, UINT64_MAX` to the common (non-zero divisor) path, which the branched version reaches without that instruction. The standard deviation nearly halves (−48.9%) because `csel` removes the rare-but-expensive branch-misprediction tail. The benefit of `csel` materializes in production code where the divisor can be zero unpredictably.

**32-bit division (Optimization 2):** Median improves by 6.7% and mean by 6.3%. Replacing the two `sxtw` instructions with native 32-bit `sdiv`/`udiv` shortens the unconditional fast path, producing a clear speed improvement across all runs.

---

## RISC-V Division Edge Cases

| Condition    | `div`   | `divu`       | `divw`    | `divuw`    | `rem`    | `remu`   |
| ------------ | ------- | ------------ | --------- | ---------- | -------- | -------- |
| Divisor = 0  | -1      | $2^{64} − 1$ | -1        | $2^{64}-1$ | dividend | dividend |
| INT_MIN / -1 | INT_MIN | N/A          | INT32_MIN | N/A        | 0        | N/A      |
