# MULHSU Microbenchmark

.text
.global _start

_start:
    li a0, 1000000          # Iteration count (1 million)
    li t0, -12345678        # RS1: negative signed value (stays negative through the chain)
    li t1, 3141592653       # RS2: large unsigned value

.loop:
    # 8x unrolled to reduce loop overhead
    mulhsu t2, t0, t1
    mulhsu t2, t2, t1
    mulhsu t2, t2, t1
    mulhsu t2, t2, t1
    mulhsu t2, t2, t1
    mulhsu t2, t2, t1
    mulhsu t2, t2, t1
    mulhsu t2, t2, t1

    addi a0, a0, -8
    bnez a0, .loop

    li a0, 0                # Return code 0
    li a7, 93               # sys_exit (Linux syscall number)
    ecall
