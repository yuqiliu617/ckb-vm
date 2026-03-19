# SHxADD / ADDUW Microbenchmark
#
# Build example:
# riscv64-unknown-elf-as -march=rv64imc_zba -o shadd_microbench.o shadd_microbench.S
# riscv64-unknown-elf-ld -o shadd_microbench shadd_microbench.o

.text
.global _start

_start:
    li a0, 1000000          # Iteration count (1 million)
    li t0, 123456789        # RS1 base value
    li t1, 987654321        # RS2 addend

.loop:
    # 8x unrolled to reduce loop overhead.
    # Covers all 7 SHxADD/ADDUW instructions (sh1add appears twice to fill 8 slots).
    add.uw   t2, t0, t1
    sh1add.uw t2, t2, t1
    sh2add.uw t2, t2, t1
    sh3add.uw t2, t2, t1
    sh1add  t2, t2, t1
    sh2add  t2, t2, t1
    sh3add  t2, t2, t1
    sh1add  t2, t2, t1

    addi a0, a0, -8
    bnez a0, .loop

    li a0, 0                # Return code 0
    li a7, 93               # sys_exit (Linux syscall number)
    ecall
