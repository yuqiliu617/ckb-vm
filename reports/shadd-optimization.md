# SHxADD / ADDUW Instruction Optimization on AArch64

## Summary

Eliminated redundant pre-shift `mov` and `lsl` instructions from 7 Zba extension handlers (ADDUW, SH1ADD, SH2ADD, SH3ADD, SH1ADDUW, SH2ADDUW, SH3ADDUW) by exploiting ARM64's extended-register and shifted-register `add` forms.

## Background

The RISC-V Zba extension provides shift-and-add instructions in two families:

- **SHxADD** (64-bit): `rd = (rs1 << N) + rs2` for N ∈ {1, 2, 3}
- **SHxADDUW / ADDUW** (32-bit): `rd = (zero_extend(rs1[31:0]) << N) + rs2` for N ∈ {0, 1, 2, 3}

ARM64 `add` natively supports both patterns via operand modifiers:

- **Shifted register**: `add Xd, Xn, Xm, LSL #N` — computes `Xn + (Xm << N)` in one instruction
- **Extended register**: `add Xd, Xn, Wm, UXTW #N` — zero-extends `Wm` to 64 bits, shifts left by N, then adds to `Xn`

## Before / After

**Before** (SH2ADDUW — representative):
```asm
  ldr RS1, REGISTER_ADDRESS(RS1)
  ldr RS2, REGISTER_ADDRESS(RS2)
  mov RS1w, RS1w              ← zero-extend to 64 bits
  lsl RS1, RS1, 2             ← shift left by 2
  add RS1, RS1, RS2           ← add RS2
```

**After** (SH2ADDUW):
```asm
  ldr RS1, REGISTER_ADDRESS(RS1)
  ldr RS2, REGISTER_ADDRESS(RS2)
  add RS1, RS2, RS1, uxtw #2  ← zero-extend RS1w, shift by 2, add RS2 — all in one
```

**Before** (SH2ADD — representative):
```asm
  ldr RS1, REGISTER_ADDRESS(RS1)
  ldr RS2, REGISTER_ADDRESS(RS2)
  lsl RS1, RS1, 2             ← shift left by 2
  add RS1, RS1, RS2           ← add RS2
```

**After** (SH2ADD):
```asm
  ldr RS1, REGISTER_ADDRESS(RS1)
  ldr RS2, REGISTER_ADDRESS(RS2)
  add RS1, RS2, RS1, lsl #2   ← shift RS1 by 2, add RS2 — all in one
```

### Instruction savings per handler

| Instruction | Before                            | After                        | Saved |
| ----------- | --------------------------------- | ---------------------------- | ----- |
| `ADDUW`     | `mov RS1w, RS1w` + `add`          | `add RS1, RS2, RS1, uxtw`    | −1    |
| `SH1ADD`    | `lsl RS1, RS1, 1` + `add`         | `add RS1, RS2, RS1, lsl #1`  | −1    |
| `SH2ADD`    | `lsl RS1, RS1, 2` + `add`         | `add RS1, RS2, RS1, lsl #2`  | −1    |
| `SH3ADD`    | `lsl RS1, RS1, 3` + `add`         | `add RS1, RS2, RS1, lsl #3`  | −1    |
| `SH1ADDUW`  | `mov` + `lsl RS1, RS1, 1` + `add` | `add RS1, RS2, RS1, uxtw #1` | −2    |
| `SH2ADDUW`  | `mov` + `lsl RS1, RS1, 2` + `add` | `add RS1, RS2, RS1, uxtw #2` | −2    |
| `SH3ADDUW`  | `mov` + `lsl RS1, RS1, 3` + `add` | `add RS1, RS2, RS1, uxtw #3` | −2    |

## Why This Is Safe

- `add Xd, Xn, Wm, UXTW #N`: ARM64 guarantees that `Wm` is zero-extended to 64 bits before shifting, matching RISC-V's `zero_extend(rs1[31:0])` semantics exactly.
- `add Xd, Xn, Xm, LSL #N`: ARM64 applies a 64-bit left shift, matching RISC-V's 64-bit SHxADD semantics.
- The shift amounts 1, 2, 3 are immediate constants embedded in the instruction encoding; no runtime masking is needed.
- The destination register (`RS1`) is the same as before, so `WRITE_RD` is unchanged.

## Benchmark

[tests/programs/shadd_microbench.S](../tests/programs/shadd_microbench.S) — 125K iterations × 8 unrolled instructions, covering all 7 handlers (SH1ADD appears twice). Instructions are chained through `t2` to serialize execution and measure handler latency.

[benches/shadd_benchmark.rs](../benches/shadd_benchmark.rs) — Criterion benchmark; reports mean, median, standard deviation, and 95% confidence intervals.

```bash
cargo bench --features=asm shadd_microbench
```

### Results

Benchmark environment: Aliyun `ecs.g8y.small`, YiTian 710 (1 core), 4 GB RAM

|         | Before                       | After                        | Change                    |
| ------- | ---------------------------- | ---------------------------- | ------------------------- |
| Mean    | [2.622, **2.637**, 2.655] ms | [2.426, **2.437**, 2.451] ms | [−8.4%, **−7.6%**, −6.8%] |
| Median  | [2.599, **2.601**, 2.603] ms | [2.408, **2.409**, 2.411] ms | [−7.5%, **−7.4%**, −7.3%] |
| Std Dev | 0.269 ms                     | 0.203 ms                     | −24.6%                    |
| MAD     | 25.3 µs                      | 20.2 µs                      | −20.1%                    |

### Interpretation

Mean and median both improve by approximately **7.5%**. This is consistent with the instruction savings: the UW variants each lose 2 instructions (saving 33% of the non-load body), and the regular SHxADD variants each lose 1 instruction. The microbench is evenly split between the two families (4 UW + 4 non-UW), so the average saving is ~1.5 instructions per handler, or roughly 25% of the pre-optimization body.

The standard deviation drops by 24.6% and MAD by 20.1%, indicating lower run-to-run variability alongside the throughput gain.
