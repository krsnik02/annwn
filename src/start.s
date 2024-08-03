.section .text.init
.global _start

_start:
    # set gp
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop

    # set sp
    la sp, _sstack

    # move data to ram
    la t0, _sidata
    la t1, _sdata
    la t2, _edata
1:  beq t1, t2, 2f
    ld t3, 0(t0)
    sd t3, 0(t1)
    addi t0, t0, 8
    addi t1, t1, 8
    j 1b
2:

    # zero bss
    la t1, _sbss
    la t2, _ebss
3:  beq t1, t2, 4f
    sd zero, 0(t1)
    addi t1, t1, 8
    j 3b
4:

    # jump to our Rust code
    j kmain
