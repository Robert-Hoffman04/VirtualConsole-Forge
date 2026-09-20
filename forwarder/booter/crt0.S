/*
 * crt0.S - booter entry. Entered from the loader with interrupts already
 * disabled by the loader, r3 = length in bytes of the staged DOL file.
 * Gives the booter its own stack (top of its region), then calls
 * booter_main(len). booter_main never returns.
 */
    .section .text.crt0, "ax"
    .globl  _start
_start:
    /* Belt and braces: clear MSR[EE] in case the caller didn't. */
    mfmsr   9
    rlwinm  9, 9, 0, 17, 15
    mtmsr   9
    isync

    lis     1, __stack_top@ha
    addi    1, 1, __stack_top@l
    subi    1, 1, 16            /* 16-byte aligned, room for the back chain */
    li      0, 0
    stw     0, 0(1)             /* terminate the back chain */

    bl      booter_main         /* r3 (len) is passed straight through */
1:  b       1b                  /* unreachable */

    .section .note.GNU-stack,"",@progbits
