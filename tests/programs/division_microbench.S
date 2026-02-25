# Division Microbenchmark

.text
.global _start

_start:
    li a0, 1000000          # Iteration count (1 million)
    li t0, 12345678         # Dividend
    li t1, 3                # Divisor (non-zero, common path)

.loop_normal:
    # 8x unrolled to reduce loop overhead
    div t2, t0, t1
    div t2, t2, t1
    div t2, t2, t1
    div t2, t2, t1
    div t2, t2, t1
    div t2, t2, t1
    div t2, t2, t1
    div t2, t2, t1

    addi a0, a0, -8
    bnez a0, .loop_normal

    # Exit successfully
    li a0, 0                # Return code 0
    li a7, 93               # sys_exit (Linux syscall number)
    ecall
