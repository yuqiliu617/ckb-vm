# MULHSU Instruction Optimization

## Summary

Eliminated a redundant load from the RISC-V register file in the `MULHSU` instruction handler, on both x86-64 and AArch64. The root cause was that the original RS1 value was loaded into a register, then **immediately destroyed** by `neg` / `neg %rax`, forcing a second load later to recover it for the low-word multiply. By preserving the original value in a spare register before negating, the second load and the stack operations it required are removed entirely.

The AArch64 handler also receives two additional optimizations: a `mvn` replaces `mov + eor` for the bitwise NOT, and `cinc` replaces `cset + add` for the correction step.

## Algorithm Background

`MULHSU rd, rs1, rs2` computes the upper 64 bits of the 128-bit product `rs1_signed × rs2_unsigned`.

Neither architecture provides a native signed×unsigned wide multiply, so the handler decomposes it by sign:

- **RS1 ≥ 0**: A signed×unsigned product is identical to an unsigned×unsigned product when the signed operand is non-negative. Plain `umulh` / `mulq` suffice.
- **RS1 < 0**: Use the identity:
  `high(rs1 × rs2) = ~high(|rs1| × rs2) + (low(rs1 × rs2) == 0 ? 1 : 0)`

  Concretely:
  1. Compute `high(|rs1| × rs2)` via unsigned multiply.
  2. Bitwise-invert the result.
  3. Add 1 if the low product `rs1 × rs2` is zero (a two's-complement carry correction).

The redundant load arises in step 3: the original code negated the RS1 register in place to compute `|rs1|`, which destroyed the original value. It then reloaded RS1 from the register file to compute the low product.

## Before / After

### AArch64

**Before:**

```asm
.CKB_VM_ASM_LABEL_OP_MULHSU:
  DECODE_R
  ldr TEMP2, REGISTER_ADDRESS(RS1)  // Load RS1
  tst TEMP2, TEMP2
  bpl .mulhsu_positive_branch
  neg TEMP2, TEMP2                  // Destroys original RS1 value
  ldr TEMP3, REGISTER_ADDRESS(RS2)
  umulh TEMP1, TEMP2, TEMP3
  mov TEMP4, -1
  eor TEMP1, TEMP1, TEMP4           // Bitwise NOT via mov + eor
  ldr TEMP2, REGISTER_ADDRESS(RS1)  // Redundant load: recover RS1 for low multiply
  mul TEMP2, TEMP2, TEMP3
  tst TEMP2, TEMP2
  cset TEMP2, eq
  add TEMP1, TEMP1, TEMP2           // Correction via cset + add
  WRITE_RD(TEMP1)
  NEXT_INST
.mulhsu_positive_branch:
  ldr TEMP3, REGISTER_ADDRESS(RS2)  // RS2 loaded separately in positive path
  umulh TEMP2, TEMP2, TEMP3
  WRITE_RD(TEMP2)
  NEXT_INST
```

**After:**

```asm
.CKB_VM_ASM_LABEL_OP_MULHSU:
  DECODE_R
  ldr TEMP2, REGISTER_ADDRESS(RS1)  // Load RS1 once
  ldr TEMP3, REGISTER_ADDRESS(RS2)  // Hoist RS2 load before branch (shared by both paths)
  tst TEMP2, TEMP2
  bpl .mulhsu_positive_branch
  neg TEMP4, TEMP2                  // Negate into TEMP4; TEMP2 preserves original RS1
  umulh TEMP1, TEMP4, TEMP3
  mvn TEMP1, TEMP1                  // Bitwise NOT in 1 instruction (replaces mov + eor)
  mul TEMP4, TEMP2, TEMP3           // Use preserved TEMP2 — no second load
  cmp TEMP4, 0
  cinc TEMP1, TEMP1, eq             // Correction in 1 instruction (replaces cset + add)
  WRITE_RD(TEMP1)
  NEXT_INST
.mulhsu_positive_branch:
  umulh TEMP2, TEMP2, TEMP3
  WRITE_RD(TEMP2)
  NEXT_INST
```

Changes:
- One register file load eliminated (negative path).
- RS2 load hoisted before the branch: the positive path avoids a dedicated `ldr TEMP3`.
- `mvn` replaces `mov TEMP4, -1; eor` (2 → 1 instruction).
- `cinc` replaces `cset TEMP2, eq; add TEMP1, TEMP1, TEMP2` (2 → 1 instruction).

### x86-64

**Before:**

```asm
.CKB_VM_ASM_LABEL_OP_MULHSU:
  DECODE_R
  PUSH_RD_IF_RAX
  PUSH_RD_IF_RDX
  PUSH_RS1_IF_RAX                   // no-op
  PUSH_RS1_IF_RDX                   // push %rdx (RS1 register index)
  movq REGISTER_ADDRESS(RS1), %rax
  test %rax, %rax
  jns .mulhsu_positive_branch
  neg %rax                          // Destroys RS1 value; %rdx (RS1 index) still on stack
  mulq REGISTER_ADDRESS(RS2r)       // Clobbers %rdx — RS1 index is now gone from %rdx
  xor $-1, %rdx
  movq %rdx, TEMP1
  POP_RS1_IF_RDX                    // pop %rdx — restore RS1 index to use for reload below
  POP_RS1_IF_RAX                    // no-op
  movq REGISTER_ADDRESS(RS1), %rax  // Redundant load: recover RS1 value from register file
  imulq REGISTER_ADDRESS(RS2r)
  test %rax, %rax
  setz %al
  movzbl %al, %eax
  addq %rax, TEMP1
  POP_RD_IF_RDX
  POP_RD_IF_RAX
  WRITE_RD(TEMP1)
  NEXT_INST
.mulhsu_positive_branch:
  mulq REGISTER_ADDRESS(RS2r)
  movq %rdx, TEMP1
  POP_RS1_IF_RDX                    // pop %rdx — stack balance only, value discarded
  POP_RS1_IF_RAX                    // no-op
  POP_RD_IF_RDX
  POP_RD_IF_RAX
  WRITE_RD(TEMP1)
  NEXT_INST
```

**After:**

```asm
.CKB_VM_ASM_LABEL_OP_MULHSU:
  DECODE_R
  PUSH_RD_IF_RAX
  PUSH_RD_IF_RDX
  movq REGISTER_ADDRESS(RS1), %rax
  test %rax, %rax
  jns .mulhsu_positive_branch
  movq %rax, TEMP2                  // Save RS1 value to TEMP2 (%r10) before negating
  neg %rax
  mulq REGISTER_ADDRESS(RS2r)       // Clobbers %rdx — but we no longer need RS1 index
  xor $-1, %rdx
  movq %rdx, TEMP1
  movq TEMP2, %rax                  // Restore RS1 value from register — no memory load
  imulq REGISTER_ADDRESS(RS2r)
  test %rax, %rax
  setz %al
  movzbl %al, %eax
  addq %rax, TEMP1
  POP_RD_IF_RDX
  POP_RD_IF_RAX
  WRITE_RD(TEMP1)
  NEXT_INST
.mulhsu_positive_branch:
  mulq REGISTER_ADDRESS(RS2r)
  movq %rdx, TEMP1
  POP_RD_IF_RDX
  POP_RD_IF_RAX
  WRITE_RD(TEMP1)
  NEXT_INST
```

Changes:
- `PUSH_RS1_IF_RDX` / `POP_RS1_IF_RDX` (`push %rdx` / `pop %rdx`) removed from both paths.
- One register file load eliminated (negative path).
- Net: negative path loses 3 instructions (push, pop, memory load) and gains 2 register moves; positive path loses 2 instructions (push, pop) unconditionally.

## Why This Is Safe

- The computation is mathematically unchanged. The same values are computed in the same order; only which register holds them differs.
- On AArch64, `neg TEMP4, TEMP2` writes the negated value into `TEMP4` while leaving `TEMP2` intact. No information is lost.
- On x86-64, `TEMP2` (`%r10`) is a caller-saved scratch register not used elsewhere in the MULHSU handler, so saving RS1 there creates no conflict.

## Benchmark

`tests/programs/mulhsu_microbench.S` — 1M chained `mulhsu` instructions (125K loop iterations × 8 unrolled). The first iteration uses the initialized negative value (`-12345678`); subsequent ones chain through `t2` as RS1. Since `mulhsu(negative, large_positive)` always produces a negative high word, all iterations exercise the negative-RS1 path.

`benches/mulhsu_benchmark.rs` — Criterion benchmark; reports mean, median, standard deviation, and 95% confidence intervals.

```bash
cargo bench --features=asm mulhsu_microbench
```

### x86-64 Results

Benchmark environment: Lenovo Legion R9000P 2021H, 16 processors (AMD Ryzen 7 5800H), 64 GB RAM

|         | Before                       | After                        | Change                    |
| ------- | ---------------------------- | ---------------------------- | ------------------------- |
| Mean    | [3.894, **3.904**, 3.916] ms | [3.788, **3.796**, 3.805] ms | [−3.1%, **−2.8%**, −2.4%] |
| Median  | [3.855, **3.857**, 3.859] ms | [3.767, **3.769**, 3.772] ms | [−2.4%, **−2.3%**, −2.2%] |
| Std Dev | 0.183 ms                     | 0.138 ms                     | −24.6%                    |
| MAD     | 25.4 µs                      | 33.8 µs                      | +33.1%                    |

Mean and median improve by approximately **2.8%** and **2.3%** respectively, reflecting the reduced instruction count and elimination of the memory load on the critical (negative-RS1) path.

### AArch64 Results

Benchmark environment: Aliyun `ecs.g8y.small`, YiTian 710 (1 core), 4 GB RAM

|         | Before                       | After                        | Change                    |
| ------- | ---------------------------- | ---------------------------- | ------------------------- |
| Mean    | [3.965, **3.982**, 4.002] ms | [3.649, **3.664**, 3.682] ms | [−8.6%, **−8.0%**, −7.4%] |
| Median  | [3.934, **3.938**, 3.941] ms | [3.628, **3.630**, 3.632] ms | [−7.9%, **−7.8%**, −7.7%] |
| Std Dev | 0.301 ms                     | 0.265 ms                     | −11.9%                    |
| MAD     | 33.1 µs                      | 31.8 µs                      | −3.9%                     |

Mean and median improve by approximately **8%**, a stronger gain than x86-64. This is consistent with AArch64 receiving more changes beyond the register allocation fix: hoisting the RS2 load eliminates one memory access on the positive path, and `mvn` + `cinc` each replace a two-instruction sequence with one. The narrow 95% confidence intervals confirm the result is statistically significant.