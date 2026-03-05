# CPOP Microbenchmark
#
# Build example:
# riscv64-unknown-elf-gcc -nostdlib -static -march=rv64imac_zbb -mabi=lp64 \
#   -Wl,-e,_start -o cpop_microbench cpop_microbench.S

.text
.global _start

_start:
    li a0, 1000000          # Total operation count
    li t0, 0x123456789abcdef0
    li t1, 0x0f0f0f0f0f0f0f0f

.loop:
    # 8x unrolled to reduce loop overhead:
    # 4x cpop + 4x cpopw
    cpop  t2, t0
    cpop  t3, t1
    cpopw t4, t0
    cpopw t5, t1
    cpop  t2, t0
    cpop  t3, t1
    cpopw t4, t0
    cpopw t5, t1

    addi a0, a0, -8
    bnez a0, .loop

    # Exit successfully
    li a0, 0
    li a7, 93
    ecall
